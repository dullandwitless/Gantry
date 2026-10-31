//! Byte-range edits over a source file, applied without reformatting
//! anything that was not removed.

use std::ops::Range;

use anyhow::{bail, Result};
use proc_macro2::{LineColumn, Span};

/// A source file with a line index for converting `proc_macro2` spans
/// (1-based line, 0-based *character* column) into byte offsets.
pub struct Source {
    text: String,
    line_starts: Vec<usize>,
}

impl Source {
    pub fn new(text: String) -> Self {
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Source { text, line_starts }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn offset(&self, at: LineColumn) -> usize {
        let line_start = self.line_starts[at.line - 1];
        self.text[line_start..]
            .char_indices()
            .nth(at.column)
            .map_or(self.text.len(), |(i, _)| line_start + i)
    }

    pub fn range(&self, span: Span) -> Range<usize> {
        self.offset(span.start())..self.offset(span.end())
    }

    /// 1-based line number containing `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|&start| start <= offset)
    }

    fn line_start(&self, offset: usize) -> usize {
        self.line_starts[self.line_of(offset) - 1]
    }

    /// Offset just past the newline ending the line that contains `offset`.
    fn next_line_start(&self, offset: usize) -> usize {
        self.text[offset..]
            .find('\n')
            .map_or(self.text.len(), |i| offset + i + 1)
    }

    fn line_text(&self, start: usize) -> &str {
        let end = self.text[start..]
            .find('\n')
            .map_or(self.text.len(), |i| start + i);
        &self.text[start..end]
    }

    /// The line before the one starting at `line_start`, if any.
    fn prev_line(&self, line_start: usize) -> Option<(usize, &str)> {
        (line_start > 0).then(|| {
            let start = self.line_start(line_start - 1);
            (start, self.line_text(start))
        })
    }

    /// Widens the removal of a syntax node so it does not leave debris behind:
    /// whole lines are taken when the node owns them, along with `//` comments
    /// directly above it and a blank line that would otherwise be doubled.
    fn widen_node_removal(&self, range: Range<usize>) -> Range<usize> {
        let text = &self.text;
        let start_line = self.line_start(range.start);
        let rest = &text[range.end..];
        let trailing = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let after = range.end + trailing;

        let owns_lines = text[start_line..range.start].trim().is_empty()
            && (after == text.len() || text[after..].starts_with(['\n', '\r']));
        if !owns_lines {
            let preceded_by_space = text[..range.start].ends_with([' ', '\t']);
            return range.start..if preceded_by_space { after } else { range.end };
        }

        let mut start = start_line;
        let mut end = self.next_line_start(after);
        while let Some((prev_start, prev)) = self.prev_line(start) {
            let prev = prev.trim_start();
            if prev.starts_with("//") && !prev.starts_with("//!") {
                start = prev_start;
            } else {
                break;
            }
        }

        let prev = self.prev_line(start).map(|(_, line)| line.trim());
        let next = (end < text.len()).then(|| self.line_text(end).trim());
        let next_is_blank = next == Some("");
        let prev_is_blank = prev == Some("");
        if next_is_blank && (prev_is_blank || prev.is_none_or(|p| p.ends_with('{'))) {
            end = self.next_line_start(end);
        } else if prev_is_blank && next.is_none_or(|n| n.starts_with('}')) {
            start = self.line_start(start - 1);
        }
        start..end
    }
}

#[derive(Default)]
pub struct Edits {
    deletions: Vec<Range<usize>>,
    replacements: Vec<(Range<usize>, String)>,
}

impl Edits {
    pub fn is_empty(&self) -> bool {
        self.deletions.is_empty() && self.replacements.is_empty()
    }

    /// Removes a syntax node, tidying the surrounding lines.
    pub fn remove_node(&mut self, src: &Source, range: Range<usize>) {
        self.deletions.push(src.widen_node_removal(range));
    }

    /// Removes exactly `range`.
    pub fn remove(&mut self, range: Range<usize>) {
        self.deletions.push(range);
    }

    pub fn replace(&mut self, range: Range<usize>, replacement: String) {
        self.replacements.push((range, replacement));
    }

    /// Whether `range` lies inside something already being removed.
    pub fn is_removed(&self, range: &Range<usize>) -> bool {
        self.deletions
            .iter()
            .any(|d| d.start <= range.start && range.end <= d.end)
    }

    pub fn apply(mut self, text: &str) -> Result<String> {
        self.deletions.sort_by_key(|r| (r.start, r.end));
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(self.deletions.len());
        for range in self.deletions {
            match merged.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }

        // Rewrites nested inside removed code are moot.
        let mut edits: Vec<(Range<usize>, String)> = self
            .replacements
            .into_iter()
            .filter(|(r, _)| !merged.iter().any(|d| d.start <= r.start && r.end <= d.end))
            .collect();
        edits.extend(merged.into_iter().map(|r| (r, String::new())));
        edits.sort_by_key(|(r, _)| (r.start, r.end));

        let mut out = String::with_capacity(text.len());
        let mut cursor = 0;
        for (range, replacement) in edits {
            if range.start < cursor {
                bail!("overlapping edits at byte {}..{}", range.start, range.end);
            }
            out.push_str(&text[cursor..range.start]);
            out.push_str(&replacement);
            cursor = range.end;
        }
        out.push_str(&text[cursor..]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remove_node(text: &str, needle: &str) -> String {
        let src = Source::new(text.to_string());
        let start = text.find(needle).unwrap();
        let mut edits = Edits::default();
        edits.remove_node(&src, start..start + needle.len());
        edits.apply(text).unwrap()
    }

    #[test]
    fn whole_line_node_takes_its_line_and_comment() {
        let text = "fn a() {}\n\n// about b\nfn b() {}\n\nfn c() {}\n";
        assert_eq!(remove_node(text, "fn b() {}"), "fn a() {}\n\nfn c() {}\n");
    }

    #[test]
    fn first_statement_in_block_leaves_no_blank_line() {
        let text = "fn a() {\n    x();\n\n    y();\n}\n";
        assert_eq!(remove_node(text, "x();"), "fn a() {\n    y();\n}\n");
    }

    #[test]
    fn last_statement_in_block_leaves_no_blank_line() {
        let text = "fn a() {\n    y();\n\n    x();\n}\n";
        assert_eq!(remove_node(text, "x();"), "fn a() {\n    y();\n}\n");
    }

    #[test]
    fn inline_node_takes_following_space() {
        assert_eq!(remove_node("f(a, b, c)", "b, "), "f(a, c)");
    }

    #[test]
    fn multibyte_columns_map_to_bytes() {
        let src = Source::new("é = 1;\nab".to_string());
        assert_eq!(src.offset(LineColumn { line: 1, column: 1 }), 2);
        assert_eq!(src.offset(LineColumn { line: 2, column: 1 }), 9);
    }

    #[test]
    fn nested_replacements_inside_removals_are_dropped() {
        let mut edits = Edits::default();
        edits.remove(0..10);
        edits.replace(2..4, "zz".into());
        assert_eq!(edits.apply("0123456789ab").unwrap(), "ab");
    }
}
