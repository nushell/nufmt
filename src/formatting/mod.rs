//! Core formatting module for nufmt.
//!
//! Walks the Nushell AST and emits properly formatted code. The heavy
//! lifting is split across focused submodules:
//!
//! - [`engine`] — Engine state setup and command stubs
//! - [`comments`] — Comment extraction and writing
//! - [`expressions`] — Expression formatting dispatch
//! - [`calls`] — Call and argument formatting
//! - [`blocks`] — Block, pipeline, and closure formatting
//! - [`collections`] — List, record, table, and match formatting
//! - [`repair`] — Parse-error repair utilities
//! - [`garbage`] — Garbage / parse-failure detection

mod blocks;
mod calls;
mod collections;
mod comments;
mod engine;
mod expressions;
mod garbage;
mod repair;

use crate::config::Config;
use crate::format_error::FormatError;
use log::{debug, trace};
use nu_parser::parse;
use nu_protocol::{engine::StateWorkingSet, ParseError, Span};

use comments::extract_comments;
use engine::get_engine_state;
use garbage::block_contains_garbage;
use repair::{
    detect_compact_if_else_spans, detect_missing_record_comma_spans,
    detect_redundant_pipeline_subexpr_spans, is_fatal_parse_error, try_repair_parse_errors,
    ParseRepairOutcome,
};

// ─────────────────────────────────────────────────────────────────────────────
// Formatter struct
// ─────────────────────────────────────────────────────────────────────────────

/// The main formatter context that tracks indentation and other state.
pub(crate) struct Formatter<'a> {
    /// The original source bytes.
    pub(crate) source: &'a [u8],
    /// The working set for looking up blocks and other data.
    pub(crate) working_set: &'a StateWorkingSet<'a>,
    /// Configuration options.
    pub(crate) config: &'a Config,
    /// Current indentation level.
    pub(crate) indent_level: usize,
    /// Output buffer.
    pub(crate) output: Vec<u8>,
    /// Track if we're at the start of a line (for indentation).
    pub(crate) at_line_start: bool,
    /// Comments extracted from source, indexed by their end position.
    pub(crate) comments: Vec<(Span, Vec<u8>)>,
    /// Track which comments have been written.
    pub(crate) written_comments: Vec<bool>,
    /// Current position in source being processed.
    pub(crate) last_pos: usize,
    /// Track nested conditional argument formatting to preserve explicit parens.
    pub(crate) conditional_context_depth: usize,
    /// Force preserving explicit parens for subexpressions inside
    /// precedence-sensitive contexts.
    pub(crate) preserve_subexpr_parens_depth: usize,
    /// Allow compact inline record style used for repaired malformed records.
    pub(crate) allow_compact_recovered_record_style: bool,
    /// Optional upper boundary for inline comment capture inside
    /// delimited contexts (e.g. subexpressions).
    pub(crate) inline_comment_upper_bound: Option<usize>,
}

/// Command types for formatting purposes.
#[derive(Debug, Clone)]
pub(crate) enum CommandType {
    Def,
    Extern,
    Conditional,
    Let,
    Block,
    Regular,
}

impl<'a> Formatter<'a> {
    fn new(
        source: &'a [u8],
        working_set: &'a StateWorkingSet<'a>,
        config: &'a Config,
        allow_compact_recovered_record_style: bool,
    ) -> Self {
        let comments = extract_comments(source);
        let written_comments = vec![false; comments.len()];
        Self {
            source,
            working_set,
            config,
            indent_level: 0,
            output: Vec::new(),
            at_line_start: true,
            comments,
            written_comments,
            last_pos: 0,
            conditional_context_depth: 0,
            preserve_subexpr_parens_depth: 0,
            allow_compact_recovered_record_style,
            inline_comment_upper_bound: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Basic output methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Write indentation if at the start of a line.
    pub(crate) fn write_indent(&mut self) {
        if self.at_line_start {
            let indent = " ".repeat(self.config.indent * self.indent_level);
            self.output.extend(indent.as_bytes());
            self.at_line_start = false;
        }
    }

    /// Write a string to output.
    pub(crate) fn write(&mut self, s: &str) {
        self.write_indent();
        self.output.extend(s.as_bytes());
    }

    /// Write bytes to output.
    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_indent();
        self.output.extend(bytes);
    }

    /// Write a newline.
    pub(crate) fn newline(&mut self) {
        self.output.push(b'\n');
        self.at_line_start = true;
    }

    /// Write a space if not at line start and not already following whitespace
    /// or an opener.
    pub(crate) fn space(&mut self) {
        if !self.at_line_start && !self.output.is_empty() {
            if let Some(&last) = self.output.last() {
                if !matches!(last, b' ' | b'\n' | b'\t' | b'(' | b'[') {
                    self.output.push(b' ');
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Span and source helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Copy the source bytes for a span into a new `Vec`.
    ///
    /// Returns owned data so that callers can slice into it while still
    /// holding `&mut self` for output writes (splitting borrows on `self`
    /// through a method return is not possible with a `&[u8]` reference).
    pub(crate) fn get_span_content(&self, span: Span) -> Vec<u8> {
        self.source[span.start..span.end].to_vec()
    }

    /// Write the original source content for a span.
    pub(crate) fn write_span(&mut self, span: Span) {
        self.write_indent();
        self.output
            .extend_from_slice(&self.source[span.start..span.end]);
    }

    /// Write the original source content for an expression's span.
    pub(crate) fn write_expr_span(&mut self, expr: &nu_protocol::ast::Expression) {
        self.write_span(expr.span);
    }

    /// Write an attribute expression while preserving a leading `@` sigil.
    pub(crate) fn write_attribute_span(&mut self, expr: &nu_protocol::ast::Expression) {
        let mut start = expr.span.start;
        if start > 0 && self.source[start - 1] == b'@' {
            start -= 1;
        }
        self.write_span(Span {
            start,
            end: expr.span.end,
        });
    }

    /// Get the final output.
    fn finish(self) -> Vec<u8> {
        self.output
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Format an array of bytes.
///
/// Reading the file gives you a list of bytes.
pub(crate) fn format_inner(contents: &[u8], config: &Config) -> Result<Vec<u8>, FormatError> {
    format_inner_with_options(contents, config)
}

fn format_inner_with_options(contents: &[u8], config: &Config) -> Result<Vec<u8>, FormatError> {
    let engine_state = get_engine_state();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}", &parsed_block);

    let source_text = String::from_utf8_lossy(contents);
    let mut malformed_spans: Vec<Span> = working_set
        .parse_errors
        .iter()
        .map(ParseError::span)
        .collect();
    malformed_spans.extend(detect_compact_if_else_spans(&source_text));
    malformed_spans.extend(detect_missing_record_comma_spans(&source_text));
    malformed_spans.extend(detect_redundant_pipeline_subexpr_spans(&source_text));

    let has_garbage = block_contains_garbage(&working_set, &parsed_block);
    let has_fatal_parse_error = working_set.parse_errors.iter().any(is_fatal_parse_error);

    if !malformed_spans.is_empty() || has_garbage {
        if let Some(repaired) = try_repair_parse_errors(contents, &malformed_spans) {
            debug!(
                "retrying formatting after targeted parse-error repair ({} parse errors)",
                working_set.parse_errors.len()
            );
            return match repaired {
                ParseRepairOutcome::Reformat(repaired_source) => {
                    format_inner_with_options(&repaired_source, config)
                }
            };
        }
    }

    if has_fatal_parse_error && has_garbage {
        debug!(
            "skipping formatting due to fatal parse errors with garbage AST nodes ({} found)",
            working_set.parse_errors.len()
        );
        return Ok(contents.to_vec());
    }

    // Note: We don't reject files with "garbage" nodes because the parser
    // produces garbage for commands it doesn't know about (e.g., `where`, `each`)
    // when using only nu-cmd-lang context. Instead, we output original span
    // content for expressions we can't format.

    if parsed_block.pipelines.is_empty() {
        trace!("block has no pipelines!");
        debug!("File has no code to format.");
        let comments = extract_comments(contents);
        if comments.is_empty() {
            return Ok(contents.to_vec());
        }
    }

    let mut formatter = Formatter::new(contents, &working_set, config, true);

    // Write leading comments
    if let Some(first_pipeline) = parsed_block.pipelines.first() {
        if let Some(first_elem) = first_pipeline.elements.first() {
            formatter.write_comments_before(first_elem.expr.span.start);
        }
    }

    formatter.format_block(&parsed_block);

    // Write trailing comments
    let end_pos = parsed_block
        .pipelines
        .last()
        .and_then(|p| p.elements.last())
        .map(|e| e.expr.span.end)
        .unwrap_or(0);

    if end_pos > 0 {
        formatter.last_pos = end_pos;
        formatter.write_comments_before(contents.len());
    }

    Ok(formatter.finish())
}

/// Make sure there is a newline at the end of a buffer.
pub(crate) fn add_newline_at_end_of_file(out: Vec<u8>) -> Vec<u8> {
    if out.last() == Some(&b'\n') {
        out
    } else {
        let mut result = out;
        result.push(b'\n');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(input: &str) -> String {
        let config = Config::default();
        let result = format_inner(input.as_bytes(), &config).expect("formatting failed");
        String::from_utf8(result).expect("invalid utf8")
    }

    #[test]
    fn repair_patterns_do_not_mutate_double_quoted_strings() {
        let input = "let s = \"if(true){1}else{2}\"";
        let output = format(input);
        assert!(output.contains("\"if(true){1}else{2}\""));
    }

    #[test]
    fn repair_patterns_do_not_mutate_record_like_strings() {
        let input = "let s = \"{ name: Alice }\"";
        let output = format(input);
        assert!(output.contains("\"{ name: Alice }\""));
    }

    #[test]
    fn test_simple_let() {
        let input = "let x = 1";
        let output = format(input);
        assert_eq!(output, "let x = 1");
    }

    #[test]
    fn test_let_with_spaces() {
        let input = "let   x   =   1";
        let output = format(input);
        assert_eq!(output, "let x = 1");
    }

    #[test]
    fn test_simple_def() {
        let input = "def foo [] { echo hello }";
        let output = format(input);
        assert!(output.contains("def foo"));
    }

    #[test]
    fn test_pipeline() {
        let input = "ls | get name";
        let output = format(input);
        assert!(output.contains("| get"));
    }

    #[test]
    fn test_if_else() {
        let input = "if true { echo yes } else { echo no }";
        let output = format(input);
        assert!(output.contains("if true"));
        assert!(output.contains("else"));
    }

    #[test]
    fn test_for_loop() {
        let input = "for x in [1, 2, 3] { print $x }";
        let output = format(input);
        assert!(output.contains("for x in"));
        assert!(output.contains("{ print"));
    }

    #[test]
    fn test_while_loop() {
        let input = "while true { break }";
        let output = format(input);
        assert!(output.contains("while true"));
        assert!(output.contains("{ break }"));
    }

    #[test]
    fn test_closure() {
        let input = "{|x| $x * 2 }";
        let output = format(input);
        assert!(output.contains("{|x|"));
    }

    #[test]
    fn test_multiline() {
        let input = "let x = 1\nlet y = 2";
        let output = format(input);
        assert!(output.contains("let x = 1"));
        assert!(output.contains("let y = 2"));
        assert!(output.contains("\n"));
    }

    #[test]
    fn test_list_simple() {
        let input = "[1, 2, 3]";
        let output = format(input);
        assert_eq!(output, "[1, 2, 3]");
    }

    #[test]
    fn test_record_simple() {
        let input = "{a: 1, b: 2}";
        let output = format(input);
        assert!(output.contains("a: 1"));
    }

    #[test]
    fn test_comment_preservation() {
        let input = "# this is a comment\nlet x = 1";
        let output = format(input);
        assert!(output.contains("# this is a comment"));
    }

    #[test]
    fn test_idempotency_let() {
        let input = "let x = 1";
        let first = format(input);
        let second = format(&first);
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_idempotency_def() {
        let input = "def foo [x: int] { $x + 1 }";
        let first = format(input);
        let second = format(&first);
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_idempotency_if_else() {
        let input = "if true { echo yes } else { echo no }";
        let first = format(input);
        let second = format(&first);
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_idempotency_for_loop() {
        let input = "for x in [1, 2, 3] { print $x }";
        let first = format(input);
        let second = format(&first);
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn test_idempotency_complex() {
        let input = "# comment\nlet x = 1\ndef foo [] { $x }";
        let first = format(input);
        let second = format(&first);
        assert_eq!(first, second, "Formatting should be idempotent");
    }

    #[test]
    fn margin_setting_inserts_expected_toplevel_spacing_issue98() {
        let input = "def foo [] {\n    let out = 1\n    out\n}\n\ndef bar [] {\n    let out = 1\n    out\n}";
        let config = Config::new(4, 80, 2);
        let result = format_inner(input.as_bytes(), &config).expect("formatting failed");
        let output = String::from_utf8(result).expect("invalid utf8");

        let expected = "def foo [] {\n    let out = 1\n    out\n}\n\n\ndef bar [] {\n    let out = 1\n    out\n}";
        assert_eq!(output, expected);
    }
}
