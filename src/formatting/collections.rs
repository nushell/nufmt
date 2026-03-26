//! Collection formatting: lists, records, tables, and match blocks.
//!
//! Handles inline-vs-multiline layout decisions and
//! comma / whitespace normalisation for each collection type.

use super::Formatter;
use nu_protocol::{
    ast::{Expr, Expression, ListItem, MatchPattern, Pattern, RecordItem},
    Span,
};

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Lists
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a list expression, choosing inline or multiline layout.
    pub(super) fn format_list(&mut self, items: &[ListItem]) {
        if items.is_empty() {
            self.write("[]");
            return;
        }

        let uses_commas = self.list_uses_commas(items);

        let all_simple = items.iter().all(|item| match item {
            ListItem::Item(expr) | ListItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        if all_simple && items.len() <= 5 {
            self.write("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    if uses_commas {
                        self.write(", ");
                    } else {
                        self.write(" ");
                    }
                }
                self.format_list_item(item);
            }
            self.write("]");
        } else {
            self.write("[");
            self.newline();
            self.indent_level += 1;
            for item in items {
                self.write_indent();
                self.format_list_item(item);
                self.newline();
            }
            self.indent_level -= 1;
            self.write_indent();
            self.write("]");
        }
    }

    /// Detect whether items are comma-separated in the original source.
    fn list_uses_commas(&self, items: &[ListItem]) -> bool {
        if items.len() < 2 {
            return false;
        }

        let item_bounds = |item: &ListItem| match item {
            ListItem::Item(expr) | ListItem::Spread(_, expr) => (expr.span.start, expr.span.end),
        };

        let (_, mut prev_end) = item_bounds(&items[0]);
        for item in items.iter().skip(1) {
            let (start, end) = item_bounds(item);
            if start > prev_end && self.source[prev_end..start].contains(&b',') {
                return true;
            }
            prev_end = end;
        }

        false
    }

    /// Format a single list item (regular or spread).
    fn format_list_item(&mut self, item: &ListItem) {
        match item {
            ListItem::Item(expr) => self.format_expression(expr),
            ListItem::Spread(_, expr) => {
                self.write("...");
                self.format_expression(expr);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Records
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a record expression, choosing inline or multiline layout.
    pub(super) fn format_record(&mut self, items: &[RecordItem], span: Span) {
        if items.is_empty() {
            self.write("{}");
            return;
        }

        let preserve_compact = self.record_preserve_compact_style(span);

        let all_simple = items.iter().all(|item| match item {
            RecordItem::Pair(k, v) => self.is_simple_expr(k) && self.is_simple_expr(v),
            RecordItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        let has_nested_complex = items.iter().any(|item| match item {
            RecordItem::Pair(_, v) => matches!(
                &v.expr,
                Expr::Record(_)
                    | Expr::List(_)
                    | Expr::Closure(_)
                    | Expr::Block(_)
                    | Expr::StringInterpolation(_)
                    | Expr::Subexpression(_)
            ),
            RecordItem::Spread(_, _) => false,
        });

        // Records with 2+ items and complex values should be multiline when nested
        let nested_multiline = self.indent_level > 0 && items.len() >= 2 && has_nested_complex;

        if all_simple && items.len() <= 3 && !nested_multiline {
            // Inline format
            let record_start = self.output.len();
            self.write("{");
            if preserve_compact {
                self.write(" ");
            }
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.format_record_item(item, preserve_compact);
            }
            if !preserve_compact
                && self.output.len() > record_start + 1
                && self.output[record_start + 1] == b' '
            {
                self.output.remove(record_start + 1);
            }
            if preserve_compact {
                self.write(" ");
            }
            self.write("}");
        } else {
            // Multiline format
            self.write("{");
            self.newline();
            self.indent_level += 1;
            for item in items {
                self.write_indent();
                self.format_record_item(item, preserve_compact);
                // Capture inline comments on the same line as the record item.
                let item_end = match item {
                    RecordItem::Pair(_, v) => v.span.end,
                    RecordItem::Spread(_, expr) => expr.span.end,
                };
                self.write_inline_comment_bounded(item_end, None);
                self.newline();
            }
            self.indent_level -= 1;
            self.write_indent();
            self.write("}");
        }
    }

    /// Determine whether a record should preserve compact colon style
    /// (used for repaired malformed records like `{ name:Alice, age:30 }`).
    fn record_preserve_compact_style(&self, span: Span) -> bool {
        if !self.allow_compact_recovered_record_style {
            return false;
        }

        if span.end <= span.start || span.end > self.source.len() {
            return false;
        }

        let slice = &self.source[span.start..span.end];
        slice.starts_with(b"{ ")
            && slice.ends_with(b" }")
            && slice.contains(&b',')
            && !slice.windows(2).any(|window| window == b": ")
    }

    /// Format a single record item (key-value pair or spread).
    fn format_record_item(&mut self, item: &RecordItem, compact_colon: bool) {
        match item {
            RecordItem::Pair(key, value) => {
                self.format_record_key(key);
                if compact_colon {
                    self.write(":");
                } else {
                    self.write(": ");
                }
                self.format_expression(value);
            }
            RecordItem::Spread(_, expr) => {
                self.write("...");
                self.format_expression(expr);
            }
        }
    }

    /// Format a record key, trimming any surrounding whitespace from the
    /// source span.
    fn format_record_key(&mut self, key: &Expression) {
        let span_contents = self.get_span_content(key.span);

        let start = span_contents
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(span_contents.len());
        let end = span_contents
            .iter()
            .rposition(|byte| !byte.is_ascii_whitespace())
            .map(|idx| idx + 1)
            .unwrap_or(0);

        if start < end {
            self.write_bytes(&span_contents[start..end]);
        } else {
            self.format_expression(key);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Tables
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a table expression (`[[col1, col2]; [val1, val2]]`).
    pub(super) fn format_table(&mut self, columns: &[Expression], rows: &[Box<[Expression]>]) {
        self.write("[");

        // Header row
        self.write("[");
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.format_expression(col);
        }
        self.write("]");

        // Data rows
        if !rows.is_empty() {
            self.write("; ");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write("[");
                for (j, cell) in row.iter().enumerate() {
                    if j > 0 {
                        self.write(", ");
                    }
                    self.format_expression(cell);
                }
                self.write("]");
            }
        }

        self.write("]");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Match blocks
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a match block (`match $val { pattern => expr }`).
    pub(super) fn format_match_block(&mut self, matches: &[(MatchPattern, Expression)]) {
        self.write("{");
        self.newline();
        self.indent_level += 1;

        for (pattern, expr) in matches {
            self.write_indent();
            self.format_match_pattern(pattern);
            self.write(" => ");
            self.format_block_or_expr(expr);
            self.newline();
        }

        self.indent_level -= 1;
        self.write_indent();
        self.write("}");
    }

    /// Format a match pattern (value, variable, list, record, or, etc.).
    pub(super) fn format_match_pattern(&mut self, pattern: &MatchPattern) {
        match &pattern.pattern {
            Pattern::Expression(expr) => self.format_expression(expr),
            Pattern::Value(_) | Pattern::Variable(_) | Pattern::Rest(_) | Pattern::Garbage => {
                self.write_span(pattern.span);
            }
            Pattern::Or(patterns) => {
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(" | ");
                    }
                    self.format_match_pattern(p);
                }
            }
            Pattern::List(patterns) => {
                self.write("[");
                for (i, p) in patterns.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.format_match_pattern(p);
                }
                self.write("]");
            }
            Pattern::Record(entries) => {
                self.write("{");
                for (i, (key, pat)) in entries.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(key);
                    self.write(": ");
                    self.format_match_pattern(pat);
                }
                self.write("}");
            }
            Pattern::IgnoreRest => self.write(".."),
            Pattern::IgnoreValue => self.write("_"),
        }
    }
}
