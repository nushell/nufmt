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
//! - [`scan`] — String-literal and comment scanning of raw source

mod blocks;
mod calls;
mod collections;
mod comments;
mod engine;
mod expressions;
mod garbage;
mod repair;
mod scan;

use crate::config::{Config, IndentChar};
use crate::format_error::FormatError;
use log::{debug, trace};
use nu_parser::parse;
use nu_protocol::{
    ParseError, Span,
    ast::{Block, Expr, Expression, Traverse},
    engine::StateWorkingSet,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use comments::extract_comments;
use engine::get_engine_state;
use garbage::block_contains_garbage;
use repair::{
    ErrorSpans, ParseRepairOutcome, detect_compact_if_else_spans,
    detect_missing_record_comma_spans, detect_redundant_pipeline_subexpr_spans,
    is_collection_error, is_fatal_parse_error, is_syntax_error, try_repair_parse_errors,
};
use scan::{Region, RegionKind, has_code_byte, parens_enclose, region_at, scan_regions};

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
    /// Comments extracted from source, in source order. Shared with probes.
    pub(crate) comments: Rc<[(Span, Vec<u8>)]>,
    /// Lists, records, and tables the parser had to recover from a syntax
    /// error inside them, sorted. Shared with probes.
    recovered_collections: Rc<[Span]>,
    /// Spans of the flags the parser reported as unknown, sorted. Shared
    /// with probes.
    unknown_flags: Rc<[Span]>,
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
    /// Force multiline pipeline emission in scoped contexts such as
    /// multiline subexpressions.
    pub(crate) force_pipeline_multiline_depth: usize,
    /// Layout decision for the `if`/`try` branch about to be formatted:
    /// `Some(true)` expands it even when it would fit on one line (issue
    /// #217). Taken by the first block, closure, or call formatted next, so
    /// it never reaches nested code.
    pub(crate) pending_branch_expansion: Option<bool>,
    /// Span of the expression of the pipeline element being formatted, so a
    /// subexpression can tell whether it is that whole element.
    pub(crate) pipeline_element_span: Option<Span>,
    /// Expansion decision of each `if`/`try` chain in each layout context.
    /// Shared with probe formatters.
    pub(crate) branch_expansion_cache: BranchExpansionCache,
}

/// The chain and layout context an `if`/`try` expansion decision was made
/// for. The decision measures the branches, so anything that changes their
/// width is part of the key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BranchExpansionKey {
    /// Start of the chain head (`if`/`try`).
    pub(crate) head_start: usize,
    pub(crate) indent_level: usize,
    /// Whether pipelines are forced onto several lines.
    pub(crate) force_pipeline_multiline: bool,
    /// Whether explicit subexpression parens are kept.
    pub(crate) preserve_subexpr_parens: bool,
}

/// Expansion decisions of `if`/`try` chains.
pub(crate) type BranchExpansionCache = Rc<RefCell<HashMap<BranchExpansionKey, bool>>>;

/// Command types for formatting purposes.
#[derive(Debug, Clone)]
pub(crate) enum CommandType {
    /// `def` / `def-env` / `export def` — function definition.
    Def,
    /// `extern` / `export extern` — extern declaration.
    Extern,
    /// `alias` / `export alias` — alias declaration.
    Alias,
    /// `if` / `try` — conditional with block arguments.
    Conditional,
    /// `let` / `let-env` / `mut` / `const` / `export const` — variable binding.
    Let,
    /// `for` / `while` / `loop` / `module` — block-taking loop/scope commands.
    Block,
    /// Any other command.
    Regular,
}

impl<'a> Formatter<'a> {
    /// Create a new `Formatter` for the given source bytes.
    ///
    /// `allow_compact_recovered_record_style` enables a special compact
    /// inline-record style used when formatting repaired malformed records
    /// (e.g. `{ name:Alice, age:30 }` from a missing-comma repair pass).
    fn new(
        source: &'a [u8],
        working_set: &'a StateWorkingSet<'a>,
        config: &'a Config,
        allow_compact_recovered_record_style: bool,
        block: &Block,
    ) -> Self {
        let comments: Rc<[(Span, Vec<u8>)]> = extract_comments(source).into();
        let written_comments = vec![false; comments.len()];
        let mut unknown_flags: Vec<Span> = working_set
            .parse_errors
            .iter()
            .filter(|error| matches!(error, ParseError::UnknownFlag(..)))
            .map(ParseError::span)
            .collect();
        unknown_flags.sort_unstable();
        Self {
            source,
            working_set,
            config,
            indent_level: 0,
            output: Vec::new(),
            at_line_start: true,
            comments,
            recovered_collections: recovered_collection_spans(working_set, block).into(),
            unknown_flags: unknown_flags.into(),
            written_comments,
            last_pos: 0,
            conditional_context_depth: 0,
            preserve_subexpr_parens_depth: 0,
            allow_compact_recovered_record_style,
            inline_comment_upper_bound: None,
            force_pipeline_multiline_depth: 0,
            pending_branch_expansion: None,
            pipeline_element_span: None,
            branch_expansion_cache: Rc::default(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Basic output methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Write indentation if at the start of a line.
    pub(crate) fn write_indent(&mut self) {
        if self.at_line_start {
            match self.config.indent_char {
                IndentChar::Space => {
                    let num_spaces = self.config.indent * self.indent_level;
                    self.output.reserve(num_spaces);
                    for _ in 0..num_spaces {
                        self.output.push(b' ');
                    }
                }
                IndentChar::Tab => {
                    self.output.reserve(self.indent_level);
                    for _ in 0..self.indent_level {
                        self.output.push(b'\t');
                    }
                }
            }
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
        if !self.at_line_start
            && !self.output.is_empty()
            && let Some(&last) = self.output.last()
            && !matches!(last, b' ' | b'\n' | b'\t' | b'(' | b'[')
        {
            self.output.push(b' ');
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
    ///
    /// Comments inside the span are written with it, so they are marked as
    /// written to keep them from being emitted a second time.
    pub(crate) fn write_span(&mut self, span: Span) {
        self.write_indent();
        self.output
            .extend_from_slice(&self.source[span.start..span.end]);
        self.mark_comments_written_in_span(span.start, span.end);
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

    /// Return `true` when the parser had to recover the list, record, or
    /// table spanning `span` from a syntax error (see
    /// `recovered_collection_spans`), so its items no longer match its
    /// source.
    pub(crate) fn is_recovered_collection(&self, span: Span) -> bool {
        self.recovered_collections.binary_search(&span).is_ok()
    }

    /// Return the spans of the flags the parser reported as unknown that lie
    /// inside `start..end`.
    pub(crate) fn unknown_flags_in(&self, start: usize, end: usize) -> impl Iterator<Item = &Span> {
        let first = self
            .unknown_flags
            .partition_point(|flag| flag.start < start);
        self.unknown_flags[first..]
            .iter()
            .take_while(move |flag| flag.start < end)
            .filter(move |flag| flag.end <= end)
    }

    /// Get the final output.
    fn finish(self) -> Vec<u8> {
        self.output
    }

    /// Run a formatting closure in an isolated probe formatter and return the
    /// output bytes without affecting `self`.
    ///
    /// Useful for measuring the rendered length of an expression before
    /// deciding whether to use inline or multiline layout. The probe starts
    /// from a fresh layout state but shares what is known about the source,
    /// so making one costs no rescan of the file.
    pub(crate) fn probe_format<F>(&self, f: F) -> Vec<u8>
    where
        F: FnOnce(&mut Formatter<'_>),
    {
        let mut probe = Formatter {
            source: self.source,
            working_set: self.working_set,
            config: self.config,
            indent_level: 0,
            output: Vec::new(),
            at_line_start: true,
            comments: Rc::clone(&self.comments),
            recovered_collections: Rc::clone(&self.recovered_collections),
            unknown_flags: Rc::clone(&self.unknown_flags),
            // Inherit the parent's comment-emission cursor so a probe never
            // re-vacuums comments that precede the current position (they
            // are already emitted / owned by the parent). Without this,
            // rendering a parenthesized match-arm guard in a probe flushes
            // every earlier standalone comment inside the guard's `(`
            // (issue: match-guard comment hoist).
            written_comments: self.written_comments.clone(),
            last_pos: self.last_pos,
            conditional_context_depth: 0,
            preserve_subexpr_parens_depth: 0,
            allow_compact_recovered_record_style: self.allow_compact_recovered_record_style,
            inline_comment_upper_bound: None,
            force_pipeline_multiline_depth: 0,
            pending_branch_expansion: None,
            pipeline_element_span: None,
            branch_expansion_cache: Rc::clone(&self.branch_expansion_cache),
        };
        f(&mut probe);
        probe.output
    }
}

/// Return the spans of the lists, records, and tables that hold a syntax
/// error, sorted.
///
/// A malformed item (`["x"; "y"]`, `{ a: 2 b }`) counts for the innermost
/// collection around it, so `{a: 1, b: ["x"; "y"]}` keeps only the list as
/// written and still formats the record. Any other syntax error, such as an
/// unclosed delimiter, may have misled the parser about every collection
/// around it, so it counts for the outermost one.
fn recovered_collection_spans(working_set: &StateWorkingSet, block: &Block) -> Vec<Span> {
    let mut errors: Vec<(Span, bool)> = working_set
        .parse_errors
        .iter()
        .filter(|error| is_syntax_error(error))
        .map(|error| (error.span(), is_collection_error(error)))
        .collect();
    if errors.is_empty() {
        return Vec::new();
    }
    errors.sort_unstable_by_key(|(span, _)| *span);

    let mut collections = Vec::new();
    block.flat_map(
        working_set,
        &|expr: &Expression| match expr.expr {
            Expr::List(_) | Expr::Record(_) | Expr::Table(_) => vec![expr.span],
            _ => Vec::new(),
        },
        &mut collections,
    );
    // Outer collections first, so that each one is open before the
    // collections nested in it.
    collections.sort_unstable_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    // Sweep the errors in source order, keeping the chain of collections
    // open at the current error. AST spans nest, so the chain's ends never
    // grow from outermost to innermost.
    let mut recovered = Vec::new();
    let mut open: Vec<Span> = Vec::new();
    let mut next = 0;
    for (error, is_local) in errors {
        while let Some(&collection) = collections.get(next).filter(|c| c.start <= error.start) {
            while open.last().is_some_and(|last| last.end <= collection.start) {
                open.pop();
            }
            open.push(collection);
            next += 1;
        }
        let enclosing = &open[..open.partition_point(|collection| collection.end >= error.end)];
        let owner = if is_local {
            enclosing.last()
        } else {
            enclosing.first()
        };
        recovered.extend(owner);
    }
    recovered.sort_unstable();
    recovered.dedup();
    recovered
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Format raw Nushell source bytes into their canonical form.
pub(crate) fn format_inner(contents: &[u8], config: &Config) -> Result<Vec<u8>, FormatError> {
    format_inner_with_options(contents, config)
}

/// Core formatting pass: parse, optionally repair, then format.
///
/// Called recursively after a repair pass when parse errors are detected.
fn format_inner_with_options(contents: &[u8], config: &Config) -> Result<Vec<u8>, FormatError> {
    let engine_state = get_engine_state();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}", parsed_block);

    let source_text = String::from_utf8_lossy(contents);
    // Only syntax errors trigger repairs. Names nufmt can't resolve, type
    // mismatches, and similar errors say nothing about the syntax nearby.
    let parse_error_spans: Vec<Span> = working_set
        .parse_errors
        .iter()
        .filter(|error| is_syntax_error(error))
        .map(ParseError::span)
        .collect();
    // A missing record comma is itself a syntax error, so missing-comma
    // repair is limited to records next to one. An unrelated error elsewhere
    // in the file must not rewrite valid records (issue #220).
    let mut malformed_spans = parse_error_spans.clone();
    if !parse_error_spans.is_empty() {
        let errors = ErrorSpans::new(&parse_error_spans);
        malformed_spans.extend(
            detect_missing_record_comma_spans(&source_text)
                .into_iter()
                .filter(|span| errors.touches(*span)),
        );
    }
    malformed_spans.extend(detect_compact_if_else_spans(&source_text));
    malformed_spans.extend(detect_redundant_pipeline_subexpr_spans(&source_text));

    let has_garbage = block_contains_garbage(&working_set, &parsed_block);
    let has_fatal_parse_error = working_set.parse_errors.iter().any(is_fatal_parse_error);

    if (!malformed_spans.is_empty() || has_garbage)
        && let Some(repaired) =
            try_repair_parse_errors(contents, &malformed_spans, &parse_error_spans)
    {
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

    let mut formatter = Formatter::new(contents, &working_set, config, true, &parsed_block);

    // A script whose statements use `$in` comes back as one `Collect`
    // element around the whole file; its statements are inside.
    let body = formatter
        .in_collect_body(&parsed_block)
        .unwrap_or(&parsed_block);

    // Write leading comments
    if let Some(first_pipeline) = body.pipelines.first()
        && let Some(first_elem) = first_pipeline.elements.first()
    {
        formatter.write_comments_before(first_elem.expr.span.start);
    }

    formatter.format_block(&parsed_block);

    // Write trailing comments. A file without code still has its comments,
    // which must all be kept (issue #231).
    let end_pos = body
        .pipelines
        .last()
        .and_then(|p| p.elements.last())
        .map(|e| e.expr.span.end)
        .unwrap_or(0);

    if end_pos > 0 || parsed_block.pipelines.is_empty() {
        formatter.last_pos = end_pos;
        formatter.write_comments_before(contents.len());
    }

    Ok(postprocess_formatted_output(formatter.finish()))
}

/// Apply post-formatting fixups that are easier to handle on the rendered
/// text than in the AST walk (e.g. collapsing redundant assignment-pipeline
/// parentheses and normalising closure `{|` spacing).
fn postprocess_formatted_output(output: Vec<u8>) -> Vec<u8> {
    // Output that is not valid UTF-8 is left alone, so its bytes are never
    // replaced.
    let Ok(text) = std::str::from_utf8(&output) else {
        return output;
    };
    let mut changed = false;
    let string_regions: Vec<Region> = scan_regions(text.as_bytes())
        .into_iter()
        .filter(|region| region.kind == RegionKind::String)
        .collect();
    let mut rebuilt = String::with_capacity(text.len());
    let mut line_start = 0;

    for line in text.split_inclusive('\n') {
        let (line_body, line_end) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };

        // A line that begins inside a multiline string starts with string
        // content. Only the code after the string's closing quote, if any,
        // is normalized.
        let string_tail = region_at(&string_regions, line_start)
            .filter(|region| region.start < line_start)
            .map_or(0, |region| (region.end - line_start).min(line_body.len()));
        line_start += line.len();
        let (string_tail, code) = line_body.split_at(string_tail);

        let mut normalized = normalize_redundant_assignment_pipeline_parens(code);
        let closure_normalized = normalize_closure_pipe_spacing(&normalized);
        if closure_normalized != normalized {
            changed = true;
            normalized = closure_normalized;
        }

        if normalized != code {
            changed = true;
        }

        rebuilt.push_str(string_tail);
        rebuilt.push_str(&normalized);
        rebuilt.push_str(line_end);
    }

    if changed {
        rebuilt.into_bytes()
    } else {
        output
    }
}

/// Remove redundant outer parentheses around a pipeline on the RHS of a
/// `let`/`mut`/`const` assignment (e.g. `let x = (a | b)` → `let x = a | b`).
fn normalize_redundant_assignment_pipeline_parens(line: &str) -> String {
    let trimmed_start = line.trim_start();
    let is_let_like = trimmed_start.starts_with("let ")
        || trimmed_start.starts_with("let-env ")
        || trimmed_start.starts_with("mut ")
        || trimmed_start.starts_with("const ")
        || trimmed_start.starts_with("export const ");
    if !is_let_like {
        return line.to_string();
    }

    let Some(eq_idx) = line.find('=') else {
        return line.to_string();
    };

    // The outer parens must wrap the whole RHS: `(a | b) + (c | d)` starts
    // and ends with a paren but needs both pairs.
    let rhs = line[eq_idx + 1..].trim_start();
    if !(rhs.contains('|') && parens_enclose(rhs.as_bytes())) {
        return line.to_string();
    }

    let inner = rhs[1..rhs.len() - 1].trim();
    // Without the parens, a `;` would end the declaration.
    if inner.is_empty() || inner.starts_with('^') || has_code_byte(inner.as_bytes(), b';') {
        return line.to_string();
    }

    if rhs.contains(") and (") || rhs.contains(") or (") {
        return line.to_string();
    }

    let lhs = line[..eq_idx].trim_end();
    format!("{lhs} = {inner}")
}

/// Remove stray spaces between `{` and `|` in closure heads
/// (e.g. `{ |p|` → `{|p|`). String literals and comments are left alone.
fn normalize_closure_pipe_spacing(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut protected = scan_regions(bytes).into_iter().peekable();
    let mut idx = 0;
    let mut changed = false;

    while idx < bytes.len() {
        if let Some(region) = protected.next_if(|region| region.start == idx) {
            result.extend_from_slice(&bytes[region.start..region.end]);
            idx = region.end;
            continue;
        }

        if bytes[idx] != b'{' {
            result.push(bytes[idx]);
            idx += 1;
            continue;
        }

        result.push(b'{');
        idx += 1;

        let spaces_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() && bytes[idx] != b'\n' {
            idx += 1;
        }

        if idx < bytes.len() && bytes[idx] == b'|' {
            let has_second_pipe = bytes[idx + 1..].contains(&b'|');
            let has_closing_brace = bytes[idx + 1..].contains(&b'}');
            if has_second_pipe && has_closing_brace {
                if idx > spaces_start {
                    changed = true;
                }
                continue;
            }
        }

        result.extend_from_slice(&bytes[spaces_start..idx]);
    }

    if changed {
        String::from_utf8(result).unwrap_or_else(|_| line.to_string())
    } else {
        line.to_string()
    }
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

    /// Input that fails to parse, so the missing-record-comma repair runs.
    const PARSE_ERROR: &str = "\nlet broken = {a: 1 b";

    #[test]
    fn record_comma_repair_skips_nested_quotes_in_interpolation_issue220() {
        let output = format(&format!("print $\"(\"a:\")\"{PARSE_ERROR}"));
        assert!(output.contains("print $\"(\"a:\")\""), "{output}");

        let input = "let b = $\"($a)(curl \"https://example.com\" -q | from json)\"";
        let output = format(&format!("{input}{PARSE_ERROR}"));
        assert!(output.contains(input), "{output}");
    }

    #[test]
    fn record_comma_repair_leaves_records_away_from_parse_errors_issue220() {
        let record = "let r = {\n    a: \"x\"\n    b: (f)\n    c: 1\n}";
        let filler = "\nlet one = 1\nlet two = 2\nlet three = 3\nlet four = 4";
        let output = format(&format!("{record}{filler}{PARSE_ERROR}"));
        assert!(output.starts_with(record), "{output}");
    }

    #[test]
    fn compact_if_else_repair_skips_interpolations_and_comments_issue220() {
        let input = "print $\"(\"}else{\")\"";
        assert_eq!(format(input), input);

        let input = "# example: if(true){1}else{2}\nlet x = 1";
        assert_eq!(format(input), input);
    }

    #[test]
    fn comment_after_interpolation_with_nested_quotes_is_kept_issue220() {
        let input = "print $\"(ansi \"#ff0000\")red\" # real comment";
        assert_eq!(format(input), input);
    }

    #[test]
    fn comment_after_backtick_string_with_apostrophe_is_kept() {
        let input = "echo `it's here` # comment\nlet x = 1";
        assert_eq!(format(input), input);
    }

    #[test]
    fn deeply_nested_one_line_chains_format_in_linear_time_issue217() {
        // Measuring a branch renders the chains nested in it; without caching
        // each chain's decision this takes time exponential in the depth.
        // The parens keep each branch simple enough to need measuring, so
        // that variant probes every level (fewer levels: the parser's
        // recursion needs more stack per level).
        for (wrap, depth) in [(false, 40), (true, 20)] {
            let mut chain = "1".to_string();
            for _ in 0..depth {
                let inner = if wrap { format!("({chain})") } else { chain };
                chain = format!("if $x {{ {inner} }} else {{ 2 }}");
            }
            let output = format(&format!("let a = {chain}"));
            assert_eq!(output, format(&output));
        }
    }

    fn format_with(input: &str, config: &Config) -> String {
        let result = format_inner(input.as_bytes(), config).expect("formatting failed");
        String::from_utf8(result).expect("invalid utf8")
    }

    /// Assert that `input` formats to `expected` and that `expected` is stable.
    fn assert_formats_to(input: &str, expected: &str) {
        assert_eq!(format(input), expected, "input:\n{input}");
        assert_eq!(format(expected), expected, "not idempotent:\n{expected}");
    }

    #[test]
    fn one_line_try_catch_closure_expands_once_issue217() {
        // The expanded chain is written across lines on the next run, and its
        // `catch {|e| ... }` closure must not collapse back.
        assert_formats_to(
            "try { foo | bar } catch {|e| baz }",
            "try {\n    foo | bar\n} catch {|e|\n    baz\n}",
        );
        assert_formats_to(
            "try { 1 } catch {|e| 2 } finally {|| foo | bar }",
            "try {\n    1\n} catch {|e|\n    2\n} finally {||\n    foo | bar\n}",
        );
        // A chain the author wrote across lines keeps its layout.
        let input = "try {\n    foo\n} catch {|e| bar }";
        assert_eq!(format(input), input);
    }

    #[test]
    fn always_mode_ignores_comments_in_the_condition_issue217() {
        let config = Config {
            consistent_branches: crate::config::ConsistentBranches::Always,
            ..Config::default()
        };
        let input = "let a = if ($x # is x set?\n) { 1 } else { 2 }";
        assert_eq!(format_with(input, &config), input);
    }

    #[test]
    fn errors_from_commands_nufmt_lacks_leave_valid_collections_alone() {
        // `%ls` and `$env.X = date now` only fail because nufmt's engine has
        // no `ls` or `date`; neither is broken syntax.
        assert_formats_to(
            "let x = {\n      a: (%ls | length)\n   b: 2\n}",
            "let x = {\n    a: (%ls | length)\n    b: 2\n}",
        );
        assert_formats_to(
            "$env.config.hooks = {\n  pre_prompt: [{||   $env.LAST = date now }]\n  show:   true\n}",
            "$env.config.hooks = {\n    pre_prompt: [\n        {|| $env.LAST = date now }\n    ]\n    show: true\n}",
        );
        // No comma goes into a valid record next to a real syntax error.
        let output = format("let r = {\n    a: (f)\n    b: 1\n}\nlet t = [1; 2]");
        assert!(
            output.starts_with("let r = {\n    a: (f)\n    b: 1\n}"),
            "{output}"
        );
    }

    #[test]
    fn only_the_innermost_broken_collection_is_kept_as_written() {
        assert_formats_to(
            "let config = {\n      show_banner:   false\n    menus: [\"a\"; \"b\"]\n}",
            "let config = {\n    show_banner: false\n    menus: [\"a\"; \"b\"]\n}",
        );
    }

    #[test]
    fn hash_inside_a_word_is_not_a_comment() {
        let input =
            "^nix run nixpkgs#hello -- --greeting \"hi\nthere\"\nlet js = \"if(x){y}else{z}\"";
        assert_eq!(format(input), input);
        let input = "ls foo#bar # real";
        assert_eq!(format(input), input);
    }

    #[test]
    fn signature_types_keep_column_names() {
        let input = "def f [x: record<external-argument: int>, y: record<\"a b\": int>, z: list<external_arg>] { $x }";
        assert_eq!(format(input), input);
    }

    #[test]
    fn signature_completers_survive_spaced_short_flags_and_rest() {
        let input = "def kg [\n    kind: string@\"kinds\"\n    --namespace (-n): string@\"nu-complete kube ns\"\n    ...nodes: string@\"nodes\"\n] { 1 }";
        let output = format(input);
        for completer in ["@\"kinds\"", "@\"nu-complete kube ns\"", "@\"nodes\""] {
            assert!(output.contains(completer), "{completer} missing:\n{output}");
        }
    }

    #[test]
    fn match_arm_comments_stay_with_their_arm() {
        // A comment after a later arm on the same line belongs to that arm.
        assert_formats_to(
            "match $x {\n    1 => \"a\"  2 => \"b\" # about b\n    _ => \"c\"\n}",
            "match $x {\n    1 => \"a\"\n    2 => \"b\" # about b\n    _ => \"c\"\n}",
        );
        // A comment inside an arm is kept even when the arm's formatter
        // doesn't write it.
        let output = format("match $x {\n    3 => [[a]; [1] # c3\n    ]\n    _ => 0\n}");
        assert!(output.contains("# c3"), "{output}");
    }

    #[test]
    fn unsafe_match_patterns_stay_quoted() {
        for word in ["_", "true", "false", "null", "inf", "NaN", "infinity"] {
            let input = format!("match $x {{\n    \"{word}\" => 1\n    _ => 2\n}}");
            assert_eq!(format(&input), input);
        }
        assert_eq!(
            format("match $x {\n    \"foo\" => 1\n    _ => 2\n}"),
            "match $x {\n    foo => 1\n    _ => 2\n}"
        );
    }

    #[test]
    fn wrapped_builtin_call_keeps_percent_sigil() {
        let args = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb ccccccccccccccccccccccccccccccc";
        let output = format(&format!("%echo {args}"));
        assert!(output.starts_with("(%echo\n"), "{output}");
    }

    #[test]
    fn env_shorthand_prefix_is_kept() {
        assert_formats_to(
            "def f [] {\n    RUST_LOG=debug cargo   run\n}",
            "def f [] {\n    RUST_LOG=debug cargo run\n}",
        );
        assert_formats_to("ls | FOO=bar print b", "ls | FOO=bar print b");
    }

    #[test]
    fn statements_inside_parens_keep_their_semicolons() {
        for input in [
            "let x = (let a = 1; $a + 1)",
            "if (let z = 1; $z > 0) { print ok }",
            "let x = ((print 1; 5) | $in + 1)",
        ] {
            assert_eq!(format(input), input);
        }
    }

    #[test]
    fn reassignment_keeps_parens_around_a_command_pipeline() {
        // `$x = git ...` is rejected: a reassignment can't start with an
        // external command.
        let input = "mut b = \"\"\n$b = ((git rev-parse HEAD) | str trim)";
        assert_eq!(format(input), input);
        assert_eq!(
            format("let c = ((git rev-parse HEAD) | str trim)"),
            "let c = git rev-parse HEAD | str trim"
        );
    }

    #[test]
    fn table_comments_are_kept() {
        let input =
            "let t = [\n    [name value]; # header\n    [a 1] # first\n    # between\n    [b 2]\n]";
        assert_eq!(format(input), input);
    }

    #[test]
    fn closure_parameter_defaults_keep_their_text() {
        for input in [
            "do {|x = \"a,b\"| $x }",
            "let f = {|x = \"a|b\"| $x }",
            "let g = {|r: record<a: int, b: int>| $r }",
        ] {
            assert_eq!(format(input), input);
        }
        assert_eq!(
            format("let h = {|x:list<int>,y = \"a:b\"| $x }"),
            "let h = {|x: list<int>, y = \"a:b\"| $x }"
        );
    }

    #[test]
    fn in_pipeline_keeps_parens_as_an_argument() {
        // Dropping them would end the command at the `|`.
        let input =
            "def f [] {\n    echo ($in | length) done\n    for x in ($in | lines) { print $x }\n}";
        assert_eq!(format(input), input);
        // As a whole statement they still go (issue #82).
        assert_eq!(
            format("def f [] { ($in | length) }"),
            "def f [] { $in | length }"
        );
    }

    #[test]
    fn bodies_using_in_keep_trailing_comments_and_statements_issue232() {
        assert_formats_to(
            "def h [] {\n    let a = $in\n    print $a\n    # trailing\n}",
            "def h [] {\n    let a = $in\n    print $a\n    # trailing\n}",
        );
        assert_formats_to(
            "def f [] { let a = $in; $a }",
            "def f [] {\n    let a = $in\n    $a\n}",
        );
        assert_formats_to(
            "$in | length\n# eof comment",
            "$in | length\n# eof comment\n",
        );
    }

    #[test]
    fn comment_only_bodies_are_stable() {
        assert_formats_to(
            "let a = 1\nlet h = {|e|\n    # ignore\n}",
            "let a = 1\n\nlet h = {|e|\n    # ignore\n}",
        );
        let input = "export-env {\n    $env.CFG = {\n        # later\n    }\n    $env.LIST = [\n        # none\n    ]\n}";
        assert_eq!(format(input), input);
    }

    #[test]
    fn empty_catch_closure_does_not_expand_the_chain_issue217() {
        let output = format("try { ls } catch {|e|}");
        assert!(output.starts_with("try { ls } catch {|e|"), "{output}");
    }

    #[test]
    fn code_after_a_multiline_string_is_normalized() {
        assert_eq!(
            format("let s = \"abc\ndef\" | each { |x| $x }"),
            "let s = \"abc\ndef\" | each {|x| $x }"
        );
    }

    #[test]
    fn chain_decision_from_indent_zero_probe_is_not_reused_issue217() {
        // Aligned match arms are measured in a probe at indent 0, where this
        // chain fits; at its real indent the first branch has to wrap.
        let words = "a".repeat(58);
        let input = format!(
            "def outer [] {{\n    def inner [] {{\n        match $y {{\n            1  => (if $x {{ echo {words} b c }} else {{ 2 }})\n            22 => 3\n        }}\n    }}\n}}"
        );
        let output = format(&input);
        assert!(output.contains("} else {\n"), "{output}");
        assert!(!output.contains("} else { 2 }"), "{output}");
    }

    #[test]
    fn parenthesized_if_after_else_starts_its_own_chain_issue217() {
        let input = "let d = if $x { 1 } else (if $y { 1 } else { ls | each {|f| $f } })";
        let expected =
            "let d = if $x { 1 } else (if $y {\n    1\n} else {\n    ls | each {|f| $f }\n})";
        assert_eq!(format(input), expected);
        assert_eq!(
            format(expected),
            expected,
            "formatting should be idempotent"
        );
    }

    #[test]
    fn call_with_unknown_flag_is_kept_as_written() {
        // Nushell 0.116 removed `do -p` and leaves unknown flags out of the
        // AST, so formatting the call from the AST would delete them.
        let input = "def f [] {\n  do -p { ls }\n  let   y = 1\n}";
        assert_eq!(
            format(input),
            "def f [] {\n    do -p { ls }\n    let y = 1\n}"
        );
        assert_eq!(format("return -1-1"), "return -1-1");
    }

    #[test]
    fn collections_with_syntax_errors_are_kept_as_written() {
        for input in [
            "{ a: 2 b }",
            "{a: 1 + 1}",
            "{a: =}",
            "[[a b];]",
            "[\"w\"; \"a\"]",
        ] {
            assert_eq!(format(input), input);
        }
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
    fn declaration_spacing_uses_formatted_layout() {
        let table = concat!(
            "let report = [\n",
            "    [name, department, salary];\n",
            "    [\"Alice\", \"Engineering\", 100000]\n",
            "    [\"Bob\", \"Marketing\", 80000]\n",
            "] # table",
        );
        for config in [
            Config::default(),
            Config::new(4, 80, 0),
            Config::new(4, 80, 1),
            Config::new(4, 80, 2),
        ] {
            for (before_comment, after_comment) in [("", ""), ("# report\n", "# after\n")] {
                let input = format!(
                    "let before = 1\n{before_comment}\
                     let report = [[name, department, salary]; \
                     [\"Alice\", \"Engineering\", 100000], [\"Bob\", \"Marketing\", 80000]] # table\n\
                     {after_comment}let after = 2\nlet end = 3"
                );
                let separator = "\n".repeat(config.margin + 1);
                let expected = format!(
                    "let before = 1{separator}{before_comment}{table}{separator}\
                     {after_comment}let after = 2\nlet end = 3"
                );
                assert_eq!(format_with(&input, &config), expected, "input:\n{input}");
                assert_eq!(
                    format_with(&expected, &config),
                    expected,
                    "not idempotent:\n{expected}"
                );
            }
        }
    }

    #[test]
    fn collapsed_declarations_stay_in_the_same_group() {
        for (input, expected) in [("[\n]", "[]"), ("{\n}", "{}"), ("[\n    1\n]", "[1]")] {
            assert_formats_to(
                &format!("let before = 1\nlet value = {input}\nlet after = 2"),
                &format!("let before = 1\nlet value = {expected}\nlet after = 2"),
            );
        }
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
