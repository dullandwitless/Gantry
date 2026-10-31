//! `cfg` predicates: parsing, partial evaluation against a set of features
//! known to be disabled, and rendering back to source.

use std::collections::BTreeSet;
use std::fmt;

use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Lit, Meta, Token};

/// A `cfg` predicate as written in source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pred {
    /// `feature = "name"`
    Feature(String),
    /// Any predicate we do not reason about (`test`, `unix`, `target_os = "linux"`, ...),
    /// kept as its rendered source.
    Opaque(String),
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
}

/// The result of partially evaluating a predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Folded {
    True,
    False,
    /// Depends on features or settings we know nothing about.
    Residual(Pred),
}

impl Pred {
    pub fn parse(meta: &Meta) -> syn::Result<Pred> {
        match meta {
            Meta::Path(path) => Ok(Pred::Opaque(path.to_token_stream().to_string())),
            Meta::NameValue(nv) if nv.path.is_ident("feature") => match &nv.value {
                Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) => Ok(Pred::Feature(s.value())),
                other => Err(syn::Error::new_spanned(other, "expected a string literal")),
            },
            Meta::NameValue(_) => Ok(Pred::Opaque(render_meta(meta))),
            Meta::List(list) => {
                let args = list
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
                    .iter()
                    .map(Pred::parse)
                    .collect::<syn::Result<Vec<_>>>()?;
                let name = list.path.get_ident().map(|ident| ident.to_string());
                match name.as_deref() {
                    Some("all") => Ok(Pred::All(args)),
                    Some("any") => Ok(Pred::Any(args)),
                    Some("not") => match <[Pred; 1]>::try_from(args) {
                        Ok([inner]) => Ok(Pred::Not(Box::new(inner))),
                        Err(_) => Err(syn::Error::new_spanned(
                            list,
                            "`not` takes exactly one predicate",
                        )),
                    },
                    _ => Ok(Pred::Opaque(render_meta(meta))),
                }
            }
        }
    }

    /// Whether any `feature = "..."` leaf names one of `features`.
    pub fn mentions_any(&self, features: &BTreeSet<String>) -> bool {
        match self {
            Pred::Feature(name) => features.contains(name),
            Pred::Opaque(_) => false,
            Pred::All(preds) | Pred::Any(preds) => preds.iter().any(|p| p.mentions_any(features)),
            Pred::Not(inner) => inner.mentions_any(features),
        }
    }

    /// Evaluates the predicate assuming every feature in `disabled` is off,
    /// simplifying whatever cannot be decided.
    pub fn fold(&self, disabled: &BTreeSet<String>) -> Folded {
        match self {
            Pred::Feature(name) if disabled.contains(name) => Folded::False,
            Pred::Feature(_) | Pred::Opaque(_) => Folded::Residual(self.clone()),
            Pred::Not(inner) => match inner.fold(disabled) {
                Folded::True => Folded::False,
                Folded::False => Folded::True,
                Folded::Residual(Pred::Not(double)) => Folded::Residual(*double),
                Folded::Residual(p) => Folded::Residual(Pred::Not(Box::new(p))),
            },
            Pred::All(preds) => fold_junction(preds, disabled, Folded::False, Pred::All),
            Pred::Any(preds) => fold_junction(preds, disabled, Folded::True, Pred::Any),
        }
    }
}

/// Folds `all(..)` / `any(..)`. `absorbing` is the value that decides the
/// whole junction (`False` for `all`, `True` for `any`); its opposite is the
/// identity and is dropped.
fn fold_junction(
    preds: &[Pred],
    disabled: &BTreeSet<String>,
    absorbing: Folded,
    make: fn(Vec<Pred>) -> Pred,
) -> Folded {
    let mut residual = Vec::new();
    for pred in preds {
        match pred.fold(disabled) {
            Folded::Residual(p) => residual.push(p),
            decided if decided == absorbing => return absorbing,
            _identity => {}
        }
    }
    match <[Pred; 1]>::try_from(residual) {
        Ok([only]) => Folded::Residual(only),
        Err(residual) if residual.is_empty() => match absorbing {
            Folded::False => Folded::True,
            _ => Folded::False,
        },
        Err(residual) => Folded::Residual(make(residual)),
    }
}

fn render_meta(meta: &Meta) -> String {
    // `quote` renders `target_os = "linux"` as-is but inserts spaces inside
    // paths and argument lists; normalise the common cases.
    meta.to_token_stream()
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" ,", ",")
}

impl fmt::Display for Pred {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn list(f: &mut fmt::Formatter<'_>, name: &str, preds: &[Pred]) -> fmt::Result {
            write!(f, "{name}(")?;
            for (i, pred) in preds.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{pred}")?;
            }
            f.write_str(")")
        }
        match self {
            Pred::Feature(name) => write!(f, "feature = {name:?}"),
            Pred::Opaque(src) => f.write_str(src),
            Pred::All(preds) => list(f, "all", preds),
            Pred::Any(preds) => list(f, "any", preds),
            Pred::Not(inner) => write!(f, "not({inner})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fold(src: &str) -> Folded {
        let meta: Meta = syn::parse_str(src).unwrap();
        let disabled = ["enterprise", "private"].map(String::from).into();
        Pred::parse(&meta).unwrap().fold(&disabled)
    }

    fn residual(src: &str) -> String {
        match fold(src) {
            Folded::Residual(p) => p.to_string(),
            other => panic!("expected residual, got {other:?}"),
        }
    }

    #[test]
    fn disabled_feature_is_false() {
        assert_eq!(fold(r#"feature = "enterprise""#), Folded::False);
        assert_eq!(fold(r#"not(feature = "private")"#), Folded::True);
    }

    #[test]
    fn all_with_disabled_member_is_false() {
        assert_eq!(
            fold(r#"all(feature = "enterprise", feature = "parquet")"#),
            Folded::False
        );
        assert_eq!(
            fold(r#"not(all(feature = "enterprise", feature = "parquet"))"#),
            Folded::True
        );
    }

    #[test]
    fn identities_are_dropped() {
        assert_eq!(
            residual(r#"all(feature = "parquet", not(feature = "private"))"#),
            r#"feature = "parquet""#
        );
        assert_eq!(residual(r#"any(feature = "private", test)"#), "test");
        assert_eq!(
            residual(r#"all(feature = "a", feature = "b", not(feature = "enterprise"))"#),
            r#"all(feature = "a", feature = "b")"#
        );
    }

    #[test]
    fn untouched_predicates_render_faithfully() {
        assert_eq!(residual(r#"target_os = "linux""#), r#"target_os = "linux""#);
        assert_eq!(
            residual(r#"not(any(unix, feature = "x"))"#),
            r#"not(any(unix, feature = "x"))"#
        );
    }

    #[test]
    fn double_negation_collapses() {
        assert_eq!(
            residual(r#"not(not(all(test, not(feature = "enterprise"))))"#),
            "test"
        );
    }

    #[test]
    fn empty_junctions_follow_cfg_semantics() {
        assert_eq!(fold("all()"), Folded::True);
        assert_eq!(fold("any()"), Folded::False);
    }

    #[test]
    fn mentions() {
        let meta: Meta = syn::parse_str(r#"all(test, not(feature = "private"))"#).unwrap();
        let pred = Pred::parse(&meta).unwrap();
        assert!(pred.mentions_any(&["private".to_string()].into()));
        assert!(!pred.mentions_any(&["enterprise".to_string()].into()));
    }
}
