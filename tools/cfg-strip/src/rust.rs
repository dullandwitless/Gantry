//! Strips feature-gated code from a single Rust source file.

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::parse::Parser;
use syn::punctuated::{Pair, Punctuated};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Arm, AttrStyle, Attribute, Block, Expr, ExprIf, ExprMacro, ExprUnary, Field, FieldPat,
    FieldValue, File, FnArg, GenericParam, ImplItem, Item, ItemMod, Lit, Macro, Meta, Stmt,
    TraitItem, UnOp, Variant,
};

use crate::edits::{Edits, Source};
use crate::pred::{Folded, Pred};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The tool could not prove it handled this; a human must.
    Review,
    /// Handled, but the result may deserve a tidy-up.
    Note,
}

#[derive(Debug)]
pub struct Diagnostic {
    /// 1-based line in the input file.
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

pub enum Outcome {
    Unchanged,
    /// The whole file is compiled out by an inner `#![cfg(..)]`.
    Removed,
    Rewritten {
        text: String,
        /// Out-of-line modules whose declarations were removed; their files go too.
        removed_modules: Vec<PathBuf>,
    },
}

pub struct FileReport {
    pub outcome: Outcome,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn strip(path: &Path, text: String, disabled: &BTreeSet<String>) -> anyhow::Result<FileReport> {
    let file: File = syn::parse_file(&text)
        .map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), e.span().start().line))?;
    let src = Source::new(text);
    let mut stripper = Stripper {
        src: &src,
        path,
        disabled,
        edits: Edits::default(),
        removed_modules: Vec::new(),
        diagnostics: Vec::new(),
        handled_cfg_macros: Vec::new(),
    };

    for attr in file
        .attrs
        .iter()
        .filter(|a| matches!(a.style, AttrStyle::Inner(_)))
    {
        if stripper.cfg_of(attr).is_some_and(|f| f == Folded::False) {
            return Ok(FileReport {
                outcome: Outcome::Removed,
                diagnostics: Vec::new(),
            });
        }
    }
    stripper.visit_file(&file);

    let Stripper {
        edits,
        removed_modules,
        diagnostics,
        ..
    } = stripper;
    // Findings inside code that is being removed anyway are moot.
    let diagnostics: Vec<_> = diagnostics
        .into_iter()
        .filter(|(range, ..)| !edits.is_removed(range))
        .map(|(range, severity, message)| Diagnostic {
            line: src.line_of(range.start),
            severity,
            message,
        })
        .collect();
    let outcome = if edits.is_empty() {
        Outcome::Unchanged
    } else {
        Outcome::Rewritten {
            text: edits.apply(src.text())?,
            removed_modules,
        }
    };
    Ok(FileReport {
        outcome,
        diagnostics,
    })
}

struct Stripper<'a> {
    src: &'a Source,
    path: &'a Path,
    disabled: &'a BTreeSet<String>,
    edits: Edits,
    removed_modules: Vec<PathBuf>,
    diagnostics: Vec<(Range<usize>, Severity, String)>,
    /// `cfg!` invocations already resolved as part of an `if` condition.
    handled_cfg_macros: Vec<Range<usize>>,
}

impl Stripper<'_> {
    fn range(&self, span: Span) -> Range<usize> {
        self.src.range(span)
    }

    fn diagnose(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push((self.range(span), Severity::Review, message.into()));
    }

    fn note(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push((self.range(span), Severity::Note, message.into()));
    }

    /// The folded predicate of a `#[cfg(..)]` attribute that mentions a
    /// disabled feature; `None` for any other attribute.
    fn cfg_of(&mut self, attr: &Attribute) -> Option<Folded> {
        if !attr.path().is_ident("cfg") {
            return None;
        }
        let pred = self.parse_pred(attr.span(), attr.parse_args::<Meta>())?;
        pred.mentions_any(self.disabled)
            .then(|| pred.fold(self.disabled))
    }

    fn parse_pred(&mut self, span: Span, meta: syn::Result<Meta>) -> Option<Pred> {
        match meta.and_then(|m| Pred::parse(&m)) {
            Ok(pred) => Some(pred),
            Err(e) => {
                self.diagnose(span, format!("unparseable cfg predicate: {e}"));
                None
            }
        }
    }

    fn is_compiled_out(&mut self, attrs: &[Attribute]) -> bool {
        attrs.iter().any(|a| self.cfg_of(a) == Some(Folded::False))
    }

    /// Removes `node` (whose span covers its attributes) if any of its
    /// `cfg`s is false. Returns whether it was removed.
    fn strip_node(&mut self, attrs: &[Attribute], node: Span) -> bool {
        let removed = self.is_compiled_out(attrs);
        if removed {
            self.edits.remove_node(self.src, self.range(node));
        }
        removed
    }

    /// Removes the compiled-out elements of a comma-separated list together
    /// with the comma that separated them from their neighbours.
    fn strip_punctuated<T: ToTokens, P: ToTokens>(
        &mut self,
        list: &Punctuated<T, P>,
        attrs_of: impl Fn(&T) -> &[Attribute],
    ) {
        let pairs: Vec<_> = list.pairs().collect();
        let removed: Vec<bool> = pairs
            .iter()
            .map(|p| self.is_compiled_out(attrs_of(p.value())))
            .collect();
        for (i, pair) in pairs.iter().enumerate() {
            if !removed[i] {
                continue;
            }
            match pair {
                Pair::Punctuated(value, punct) => {
                    let range = self.range(value.span()).start..self.range(punct.span()).end;
                    self.edits.remove_node(self.src, range);
                }
                Pair::End(value) => {
                    let value = self.range(value.span());
                    // Without a trailing comma, the last survivor's comma would
                    // dangle; take it and everything up to the removed value.
                    let survivor_comma =
                        (0..i)
                            .rev()
                            .find(|&j| !removed[j])
                            .and_then(|j| match pairs[j] {
                                Pair::Punctuated(_, punct) => Some(self.range(punct.span())),
                                Pair::End(_) => None,
                            });
                    match survivor_comma {
                        Some(comma) => self.edits.remove(comma.start..value.end),
                        None => self.edits.remove_node(self.src, value),
                    }
                }
            }
        }
    }

    /// Resolves `if cfg!(..) { .. } else { .. }` by keeping only the branch
    /// that is compiled in. Returns whether the condition was such a `cfg!`.
    fn strip_cfg_if(&mut self, expr: &ExprIf) -> bool {
        let (mac, negated) = match &*expr.cond {
            Expr::Macro(ExprMacro { mac, .. }) => (mac, false),
            Expr::Unary(ExprUnary {
                op: UnOp::Not(_),
                expr,
                ..
            }) => match &**expr {
                Expr::Macro(ExprMacro { mac, .. }) => (mac, true),
                _ => return false,
            },
            _ => return false,
        };
        if !mac.path.is_ident("cfg") {
            return false;
        }
        let Some(pred) = self.parse_pred(mac.span(), mac.parse_body::<Meta>()) else {
            return false;
        };
        if !pred.mentions_any(self.disabled) {
            return false;
        }
        let taken = match pred.fold(self.disabled) {
            Folded::True => !negated,
            Folded::False => negated,
            Folded::Residual(_) => {
                self.diagnose(
                    mac.span(),
                    "`cfg!` depends on other features; rewrite by hand",
                );
                return false;
            }
        };
        self.handled_cfg_macros.push(self.range(mac.span()));

        let whole = self.range(expr.span());
        let then_block = self.range(expr.then_branch.span());
        let else_branch = expr.else_branch.as_ref().map(|(_, e)| self.range(e.span()));
        match (taken, else_branch) {
            (true, else_branch) => {
                self.edits.remove(whole.start..then_block.start);
                if let Some(else_branch) = else_branch {
                    self.edits.remove(then_block.end..else_branch.end);
                }
                self.note(
                    expr.span(),
                    "`if cfg!` reduced to its block; consider unwrapping it",
                );
            }
            (false, Some(else_branch)) => {
                self.edits.remove(whole.start..else_branch.start);
                self.note(
                    expr.span(),
                    "`if cfg!` reduced to its else-branch; consider unwrapping it",
                );
            }
            (false, None) => self.edits.remove_node(self.src, whole),
        }
        true
    }

    /// Paths an out-of-line `mod name;` in this file can resolve to.
    fn module_paths(&self, item: &ItemMod) -> Vec<PathBuf> {
        let dir = self.path.parent().unwrap_or(Path::new("."));
        if let Some(path) = item.attrs.iter().find_map(path_attr) {
            return vec![dir.join(path)];
        }
        let is_dir_owner = matches!(
            self.path.file_name().and_then(|n| n.to_str()),
            Some("mod.rs" | "lib.rs" | "main.rs")
        );
        let base = if is_dir_owner {
            dir.to_path_buf()
        } else {
            dir.join(self.path.file_stem().unwrap_or_default())
        };
        let name = item.ident.to_string();
        vec![base.join(format!("{name}.rs")), base.join(&name)]
    }
}

fn path_attr(attr: &Attribute) -> Option<String> {
    match &attr.meta {
        Meta::NameValue(nv) if nv.path.is_ident("path") => match &nv.value {
            Expr::Lit(lit) => match &lit.lit {
                Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

macro_rules! attrs_of {
    ($node:expr, $($variant:ident)|+) => {
        match $node {
            $( $variant(n) => n.attrs.as_slice(), )+
            _ => &[],
        }
    };
}

fn item_attrs(item: &Item) -> &[Attribute] {
    use Item::*;
    attrs_of!(
        item,
        Const
            | Enum
            | ExternCrate
            | Fn
            | ForeignMod
            | Impl
            | Macro
            | Mod
            | Static
            | Struct
            | Trait
            | TraitAlias
            | Type
            | Union
            | Use
    )
}

fn impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    use ImplItem::*;
    attrs_of!(item, Const | Fn | Type | Macro)
}

fn trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    use TraitItem::*;
    attrs_of!(item, Const | Fn | Type | Macro)
}

fn expr_attrs(expr: &Expr) -> &[Attribute] {
    use Expr::*;
    attrs_of!(
        expr,
        Array
            | Assign
            | Async
            | Await
            | Binary
            | Block
            | Break
            | Call
            | Cast
            | Closure
            | Const
            | Continue
            | Field
            | ForLoop
            | Group
            | If
            | Index
            | Infer
            | Let
            | Lit
            | Loop
            | Macro
            | Match
            | MethodCall
            | Paren
            | Path
            | Range
            | RawAddr
            | Reference
            | Repeat
            | Return
            | Struct
            | Try
            | TryBlock
            | Tuple
            | Unary
            | Unsafe
            | While
            | Yield
    )
}

fn generic_param_attrs(param: &GenericParam) -> &[Attribute] {
    match param {
        GenericParam::Type(p) => &p.attrs,
        GenericParam::Lifetime(p) => &p.attrs,
        GenericParam::Const(p) => &p.attrs,
    }
}

fn fn_arg_attrs(arg: &FnArg) -> &[Attribute] {
    match arg {
        FnArg::Receiver(r) => &r.attrs,
        FnArg::Typed(t) => &t.attrs,
    }
}

impl<'ast> Visit<'ast> for Stripper<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if self.strip_node(item_attrs(item), item.span()) {
            if let Item::Mod(module) = item {
                if module.content.is_none() {
                    let paths = self.module_paths(module);
                    self.removed_modules.extend(paths);
                }
            }
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if !self.strip_node(impl_item_attrs(item), item.span()) {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        if !self.strip_node(trait_item_attrs(item), item.span()) {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        let attrs = match stmt {
            Stmt::Local(local) => local.attrs.as_slice(),
            Stmt::Macro(mac) => mac.attrs.as_slice(),
            Stmt::Expr(expr, _) => expr_attrs(expr),
            // Items are handled by `visit_item`.
            Stmt::Item(_) => &[],
        };
        if !self.strip_node(attrs, stmt.span()) {
            visit::visit_stmt(self, stmt);
        }
    }

    fn visit_arm(&mut self, arm: &'ast Arm) {
        if !self.strip_node(&arm.attrs, arm.span()) {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_fields_named(&mut self, fields: &'ast syn::FieldsNamed) {
        self.strip_punctuated(&fields.named, |f: &Field| &f.attrs);
        visit::visit_fields_named(self, fields);
    }

    fn visit_fields_unnamed(&mut self, fields: &'ast syn::FieldsUnnamed) {
        self.strip_punctuated(&fields.unnamed, |f: &Field| &f.attrs);
        visit::visit_fields_unnamed(self, fields);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.strip_punctuated(&item.variants, |v: &Variant| &v.attrs);
        visit::visit_item_enum(self, item);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        self.strip_punctuated(&expr.fields, |f: &FieldValue| &f.attrs);
        visit::visit_expr_struct(self, expr);
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        self.strip_punctuated(&expr.args, expr_attrs);
        visit::visit_expr_call(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        self.strip_punctuated(&expr.args, expr_attrs);
        visit::visit_expr_method_call(self, expr);
    }

    fn visit_expr_array(&mut self, expr: &'ast syn::ExprArray) {
        self.strip_punctuated(&expr.elems, expr_attrs);
        visit::visit_expr_array(self, expr);
    }

    fn visit_expr_tuple(&mut self, expr: &'ast syn::ExprTuple) {
        self.strip_punctuated(&expr.elems, expr_attrs);
        visit::visit_expr_tuple(self, expr);
    }

    fn visit_pat_struct(&mut self, pat: &'ast syn::PatStruct) {
        self.strip_punctuated(&pat.fields, |f: &FieldPat| &f.attrs);
        visit::visit_pat_struct(self, pat);
    }

    fn visit_signature(&mut self, sig: &'ast syn::Signature) {
        self.strip_punctuated(&sig.inputs, fn_arg_attrs);
        visit::visit_signature(self, sig);
    }

    fn visit_generics(&mut self, generics: &'ast syn::Generics) {
        self.strip_punctuated(&generics.params, generic_param_attrs);
        visit::visit_generics(self, generics);
    }

    fn visit_expr_if(&mut self, expr: &'ast ExprIf) {
        self.strip_cfg_if(expr);
        visit::visit_expr_if(self, expr);
    }

    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        let range = self.range(attr.span());
        if attr.path().is_ident("cfg") {
            match self.cfg_of(attr) {
                None => {}
                // Removed together with its node by one of the node visitors;
                // if no visitor claimed it, `strip` reports it.
                Some(Folded::False) => self.diagnostics.push((
                    range,
                    Severity::Review,
                    "compiled-out code in an unsupported position".into(),
                )),
                Some(Folded::True) => self.edits.remove_node(self.src, range),
                Some(Folded::Residual(pred)) => {
                    self.edits.replace(range, format!("#[cfg({pred})]"))
                }
            }
        } else if attr.path().is_ident("cfg_attr") {
            self.strip_cfg_attr(attr, range);
        }
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        let range = self.range(mac.span());
        if self.handled_cfg_macros.contains(&range) {
            return;
        }
        if mac.path.is_ident("cfg") {
            self.strip_cfg_macro(mac, range);
        } else if self.strip_macro_body(mac.tokens.clone()) {
            self.diagnose(
                mac.span(),
                "gated code inside a macro invocation; strip by hand",
            );
        }
    }
}

impl Stripper<'_> {
    /// Replaces a decidable `cfg!(..)` with its boolean value.
    fn strip_cfg_macro(&mut self, mac: &Macro, range: Range<usize>) {
        let Some(pred) = self.parse_pred(mac.span(), mac.parse_body::<Meta>()) else {
            return;
        };
        if !pred.mentions_any(self.disabled) {
            return;
        }
        match pred.fold(self.disabled) {
            Folded::True => self.edits.replace(range, "true".into()),
            Folded::False => self.edits.replace(range, "false".into()),
            Folded::Residual(_) => self.diagnose(
                mac.span(),
                "`cfg!` depends on other features; rewrite by hand",
            ),
        }
    }

    /// Strips what it can inside a macro body: every `{ .. }` group that
    /// parses as statements is processed like ordinary code. Returns whether
    /// gated `cfg`s remain outside such groups.
    fn strip_macro_body(&mut self, tokens: TokenStream) -> bool {
        let tokens: Vec<_> = tokens.into_iter().collect();
        let mut unhandled = false;
        for (i, tt) in tokens.iter().enumerate() {
            match tt {
                TokenTree::Ident(ident) if ident == "cfg" || ident == "cfg_attr" => {
                    let args = tokens[i + 1..]
                        .iter()
                        .find(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == '!'));
                    if let Some(TokenTree::Group(args)) = args {
                        unhandled |= args.delimiter() == Delimiter::Parenthesis
                            && self.names_disabled_feature(args.stream());
                    }
                }
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    match Block::parse_within.parse2(group.stream()) {
                        Ok(stmts) => stmts.iter().for_each(|stmt| self.visit_stmt(stmt)),
                        Err(_) => unhandled |= self.strip_macro_body(group.stream()),
                    }
                }
                TokenTree::Group(group) => unhandled |= self.strip_macro_body(group.stream()),
                _ => {}
            }
        }
        unhandled
    }

    fn names_disabled_feature(&self, tokens: TokenStream) -> bool {
        tokens.into_iter().any(|tt| match tt {
            TokenTree::Literal(lit) => {
                let lit = lit.to_string();
                self.disabled.iter().any(|f| lit == format!("\"{f}\""))
            }
            TokenTree::Group(group) => self.names_disabled_feature(group.stream()),
            _ => false,
        })
    }

    fn strip_cfg_attr(&mut self, attr: &Attribute, range: Range<usize>) {
        let args = match attr.parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        {
            Ok(args) => args,
            Err(e) => return self.diagnose(attr.span(), format!("unparseable cfg_attr: {e}")),
        };
        let mut args = args.into_iter();
        let Some(pred) = args
            .next()
            .and_then(|m| self.parse_pred(attr.span(), Ok(m)))
        else {
            return;
        };
        if !pred.mentions_any(self.disabled) {
            return;
        }
        // Keep the attributes exactly as written.
        let attrs: Vec<&str> = args
            .map(|m| &self.src.text()[self.range(m.span())])
            .collect();
        match pred.fold(self.disabled) {
            Folded::False => self.edits.remove_node(self.src, range),
            Folded::True => {
                let rendered = attrs
                    .iter()
                    .map(|a| format!("#[{a}]"))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.edits.replace(range, rendered);
            }
            Folded::Residual(pred) => {
                self.edits
                    .replace(range, format!("#[cfg_attr({pred}, {})]", attrs.join(", ")));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    fn run(input: &str) -> (String, Vec<String>) {
        let disabled = ["enterprise", "private"].map(String::from).into();
        let report = strip(Path::new("src/lib.rs"), input.to_string(), &disabled).unwrap();
        let text = match report.outcome {
            Outcome::Unchanged => input.to_string(),
            Outcome::Removed => String::new(),
            Outcome::Rewritten { text, .. } => text,
        };
        let reviews = report
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == Severity::Review)
            .map(|d| d.message)
            .collect();
        (text, reviews)
    }

    #[test]
    fn strips_items_statements_and_rewrites_attributes() {
        let (out, diagnostics) = run(indoc! {r#"
            use a::b;
            #[cfg(feature = "enterprise")]
            use ee::thing;

            #[cfg(not(feature = "enterprise"))]
            pub fn open() {}

            /// Docs travel with the item.
            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            pub fn gated() {}

            #[cfg(all(feature = "parquet", not(feature = "private")))]
            pub fn parquet() {
                let x = 1;
                // explains the gated call
                #[cfg(feature = "private")]
                secret(x);
                done(x);
            }
        "#});
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            out,
            indoc! {r#"
                use a::b;

                pub fn open() {}

                #[cfg(feature = "parquet")]
                pub fn parquet() {
                    let x = 1;
                    done(x);
                }
            "#}
        );
    }

    #[test]
    fn strips_fields_variants_and_arms_with_their_commas() {
        let (out, diagnostics) = run(indoc! {r#"
            struct S {
                a: u8,
                #[cfg(feature = "enterprise")]
                b: u8,
            }
            enum E { A, #[cfg(feature = "private")] B }
            fn f(e: E) -> S {
                match e {
                    E::A => {}
                    #[cfg(feature = "private")]
                    E::B => {}
                }
                S {
                    a: 0,
                    #[cfg(feature = "enterprise")]
                    b: 1,
                }
            }
        "#});
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            out,
            indoc! {r#"
                struct S {
                    a: u8,
                }
                enum E { A }
                fn f(e: E) -> S {
                    match e {
                        E::A => {}
                    }
                    S {
                        a: 0,
                    }
                }
            "#}
        );
    }

    #[test]
    fn strips_call_arguments() {
        let (out, diagnostics) = run(indoc! {r#"
            fn f() {
                connect(
                    db,
                    #[cfg(feature = "enterprise")]
                    jwks,
                );
                g(a, #[cfg(feature = "private")] b);
            }
        "#});
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            out,
            "fn f() {\n    connect(\n        db,\n    );\n    g(a);\n}\n"
        );
    }

    #[test]
    fn resolves_if_cfg() {
        let (out, _) = run(indoc! {r#"
            fn f() {
                if cfg!(feature = "enterprise") {
                    ee();
                }
                if !cfg!(feature = "private") { oss(); } else { ee(); }
            }
        "#});
        assert_eq!(out, "fn f() {\n    { oss(); }\n}\n");
    }

    #[test]
    fn cfg_attr_is_resolved() {
        let (out, _) = run(indoc! {r#"
            #[cfg_attr(feature = "enterprise", derive(Debug))]
            #[cfg_attr(not(feature = "enterprise"), allow(dead_code))]
            struct S;
        "#});
        assert_eq!(out, "#[allow(dead_code)]\nstruct S;\n");
    }

    #[test]
    fn folds_cfg_macros_used_as_values() {
        let (out, diagnostics) = run(indoc! {r#"
            fn f() -> bool {
                cfg!(feature = "enterprise") || !cfg!(feature = "private")
            }
        "#});
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(out, "fn f() -> bool {\n    false || !false\n}\n");
    }

    #[test]
    fn strips_inside_macro_blocks() {
        let (out, diagnostics) = run(indoc! {r#"
            fn f() {
                tokio::select! {
                    _ = a() => {
                        x();
                        #[cfg(feature = "enterprise")]
                        y();
                    }
                }
            }
        "#});
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            out,
            "fn f() {\n    tokio::select! {\n        _ = a() => {\n            x();\n        }\n    }\n}\n"
        );
    }

    #[test]
    fn flags_what_it_cannot_prove() {
        let (_, diagnostics) = run(indoc! {r#"
            fn f() {
                let on = cfg!(any(feature = "enterprise", unix));
                tokio::select! {
                    #[cfg(feature = "enterprise")]
                    _ = ee() => {}
                }
            }
        "#});
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    }

    #[test]
    fn inner_cfg_removes_the_file() {
        let disabled = ["private".to_string()].into();
        let report = strip(
            Path::new("x.rs"),
            "#![cfg(feature = \"private\")]\nfn f() {}\n".into(),
            &disabled,
        )
        .unwrap();
        assert!(matches!(report.outcome, Outcome::Removed));
    }
}
