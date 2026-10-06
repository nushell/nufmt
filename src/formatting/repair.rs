//! Parse-error repair utilities.
//!
//! When the Nushell parser emits recoverable errors (compact `if/else`,
//! missing record commas, etc.), these routines attempt to patch the
//! source text so a second parse succeeds cleanly. Repairs are
//! region-scoped and never mutate string-literal contents.

use super::scan::{Region, RegionKind, has_code_byte, parens_enclose, region_at, scan_regions};
use nu_protocol::{ParseError, Span};

/// Whether a parse error is fatal enough that formatting should be skipped.
pub(super) fn is_fatal_parse_error(error: &ParseError) -> bool {
    match error {
        ParseError::UnknownCommand(..) => true,
        ParseError::LabeledErrorWithHelp { .. } => false,
        _ => is_syntax_error(error),
    }
}

/// Messages of the [`ParseError::LabeledErrorWithHelp`] errors that report
/// malformed syntax in nu-parser 0.116: broken records, lists, tables, and
/// patterns, and a few others.
///
/// The same variant also carries errors that only appear because nufmt's
/// engine knows just the core commands, such as "External command calls
/// must be explicit in assignments" for `$x = date now` or "percent sigil
/// requires a built-in command" for `%ls`. Those say nothing about the
/// syntax, so they are not listed.
const SYNTAX_ERROR_MESSAGES: &[&str] = &[
    "Unexpected semicolon in list pattern",
    "Match guard without an expression",
    "no space between name and parameters",
    "Incomplete variable",
];

/// Messages of the syntax errors that nushell 0.116 reports for a malformed
/// item inside one record, list, or table, after which it recovers the rest.
const COLLECTION_ERROR_MESSAGES: &[&str] = &[
    "Unexpected semicolon in list",
    "Table item not list",
    "Table column name not string",
    "Unexpected token in record",
    "Incomplete record field",
    "Expected `:` after record key",
    "Unexpected token in record value",
];

/// Whether a parse error reports a malformed item of one record, list, or
/// table. Unlike an unclosed delimiter, such an error leaves the
/// collections around that one intact.
pub(super) fn is_collection_error(error: &ParseError) -> bool {
    matches!(
        error,
        ParseError::LabeledErrorWithHelp { error, .. }
            if COLLECTION_ERROR_MESSAGES.contains(&error.as_str())
    )
}

/// Whether a parse error reports broken syntax at its location: an unclosed
/// or unbalanced delimiter, a missing or extra token, or (since nushell
/// 0.116) a malformed record, list, or table.
///
/// Other errors say nothing about the syntax there, and many only appear
/// because nufmt's engine knows just the core commands: unresolved
/// variables and modules, type mismatches, or a `const` that can't be
/// evaluated without `ansi` or `path self`.
pub(super) fn is_syntax_error(error: &ParseError) -> bool {
    match error {
        ParseError::ExtraTokens(..)
        | ParseError::ExtraTokensAfterClosingDelimiter(..)
        | ParseError::UnexpectedEof(..)
        | ParseError::Unclosed(..)
        | ParseError::Unbalanced(..)
        | ParseError::IncompleteMathExpression(..)
        | ParseError::Expected(..)
        | ParseError::ExpectedWithStringMsg(..)
        | ParseError::ExpectedWithDidYouMean(..) => true,
        ParseError::LabeledErrorWithHelp { error: message, .. } => {
            SYNTAX_ERROR_MESSAGES.contains(&message.as_str()) || is_collection_error(error)
        }
        _ => false,
    }
}

/// The outcome of a successful repair pass.
pub(super) enum ParseRepairOutcome {
    /// Repaired source bytes ready for re-parsing and re-formatting.
    Reformat(Vec<u8>),
}

// ─────────────────────────────────────────────────────────────────────────────
// String-literal and comment safeguards
// ─────────────────────────────────────────────────────────────────────────────

/// Find the string literals and comments of `source` so that
/// transformations can skip their contents.
fn find_protected_ranges(source: &str) -> Vec<Region> {
    scan_regions(source.as_bytes())
}

/// Apply a transformation only to the portions of `source` outside string
/// literals and comments.
fn transform_outside_protected_ranges(
    source: &str,
    mut transform: impl FnMut(&str) -> (String, bool),
) -> (String, bool) {
    let mut output = String::with_capacity(source.len());
    let mut changed = false;
    let mut cursor = 0;

    for region in find_protected_ranges(source) {
        if cursor < region.start {
            let (transformed, seg_changed) = transform(&source[cursor..region.start]);
            output.push_str(&transformed);
            changed |= seg_changed;
        }
        output.push_str(&source[region.start..region.end]);
        cursor = region.end;
    }

    if cursor < source.len() || source.is_empty() {
        let (transformed, seg_changed) = transform(&source[cursor..]);
        output.push_str(&transformed);
        changed |= seg_changed;
    }

    (output, changed)
}

/// Return ranges of `source` that are *not* inside string literals or
/// comments.
fn unprotected_ranges(source: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = 0;

    for region in find_protected_ranges(source) {
        if cursor < region.start {
            ranges.push((cursor, region.start));
        }
        cursor = region.end;
    }

    if cursor < source.len() {
        ranges.push((cursor, source.len()));
    }

    ranges
}

// ─────────────────────────────────────────────────────────────────────────────
// Compact if/else repair
// ─────────────────────────────────────────────────────────────────────────────

/// Detect spans that likely contain compact `if/else` patterns (e.g.
/// `if(cond){body}else{body}`).
pub(super) fn detect_compact_if_else_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let patterns = ["if(", "}else{", "} else{", "}else {"];

    for (range_start, range_end) in unprotected_ranges(source) {
        let segment = &source[range_start..range_end];
        for pattern in patterns {
            for (offset, _) in segment.match_indices(pattern) {
                let idx = range_start + offset;
                spans.push(Span {
                    start: idx.saturating_sub(32),
                    end: (idx + pattern.len() + 32).min(source.len()),
                });
            }
        }
    }

    spans
}

/// Repair compact `if/else` within a single (non-string) segment.
fn repair_compact_if_else_segment(segment: &str) -> (String, bool) {
    let mut repaired = segment.to_string();
    let mut changed = false;
    let replacements = [
        ("if(", "if ("),
        ("){", ") {"),
        ("}else{", "} else {"),
        ("} else{", "} else {"),
        ("}else {", "} else {"),
    ];

    for (from, to) in replacements {
        if repaired.contains(from) {
            repaired = repaired.replace(from, to);
            changed = true;
        }
    }

    (repaired, changed)
}

/// Attempt to repair compact `if/else` patterns in `source`.
pub(super) fn try_repair_compact_if_else(source: &str) -> (String, bool) {
    transform_outside_protected_ranges(source, repair_compact_if_else_segment)
}

// ─────────────────────────────────────────────────────────────────────────────
// Missing record-comma repair
// ─────────────────────────────────────────────────────────────────────────────

/// Return whether the value whose last byte is at `value_end` runs straight
/// into a record key, as in `{name:"Alice"age:30}`.
///
/// Fields separated by whitespace (`{a: "x" b: 1}`, or one per line) are
/// valid without commas, so only a key glued to the value counts.
fn is_missing_record_comma(bytes: &[u8], value_end: usize) -> bool {
    let key_start = value_end + 1;
    if key_start >= bytes.len()
        || !(bytes[key_start].is_ascii_alphabetic() || bytes[key_start] == b'_')
    {
        return false;
    }

    let mut key_end = key_start;
    while key_end < bytes.len()
        && (bytes[key_end].is_ascii_alphanumeric()
            || bytes[key_end] == b'_'
            || bytes[key_end] == b'-')
    {
        key_end += 1;
    }

    key_end < bytes.len() && bytes[key_end] == b':'
}

fn detect_missing_record_comma_positions(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let mut insert_positions: Vec<usize> = Vec::new();

    // Only a `)` in code can close a record value; one inside a string literal
    // or comment never does.
    let check_code = |start: usize, end: usize, positions: &mut Vec<usize>| {
        for idx in start..end {
            if bytes[idx] == b')' && is_missing_record_comma(bytes, idx) {
                positions.push(idx + 1);
            }
        }
    };

    // The shared scanner understands comments (an apostrophe in "don't" is not
    // a quote) and the nested quotes of string interpolations such as
    // `$"("a:")"`, whose inner text is not a record key (issue #220).
    let mut cursor = 0;
    for region in scan_regions(bytes) {
        check_code(cursor, region.start, &mut insert_positions);

        let closing_quote = region.end - 1;
        if region.kind == RegionKind::String
            && closing_quote > region.start
            && bytes[closing_quote] == b'"'
            && is_missing_record_comma(bytes, closing_quote)
        {
            insert_positions.push(region.end);
        }

        cursor = region.end;
    }
    check_code(cursor, bytes.len(), &mut insert_positions);

    insert_positions
}

/// Detect spans around positions where record commas are likely missing.
pub(super) fn detect_missing_record_comma_spans(source: &str) -> Vec<Span> {
    detect_missing_record_comma_positions(source)
        .into_iter()
        .map(|idx| Span {
            start: idx.saturating_sub(32),
            end: (idx + 32).min(source.len()),
        })
        .collect()
}

/// Detect spans that likely contain redundant outer parentheses around
/// pipeline-leading subexpressions, such as `((pwd) | where true)`.
pub(super) fn detect_redundant_pipeline_subexpr_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();

    for (range_start, range_end) in unprotected_ranges(source) {
        let segment = &source[range_start..range_end];
        for (offset, _) in segment.match_indices("((") {
            let idx = range_start + offset;
            let line_end = source[idx..]
                .find('\n')
                .map_or(source.len(), |rel| idx + rel);
            let line_start = source[..idx].rfind('\n').map_or(0, |rel| rel + 1);
            let line = &source[idx..line_end];
            if line.contains('|')
                && line.trim_end().ends_with(')')
                && is_droppable_pipeline_wrapper(&source[line_start..line_end], idx - line_start)
            {
                spans.push(Span {
                    start: idx.saturating_sub(32),
                    end: (line_end + 32).min(source.len()),
                });
            }
        }
    }

    spans
}

/// Return whether the `((` at byte `start` of `line` opens a wrapper whose
/// outer parens can be dropped, as in `let p = ((pwd) | path join x)`.
///
/// The wrapper must be the right-hand side of a `let`/`mut`/`const`
/// declaration, its outer parens must enclose the rest of the line, and it
/// must hold a single statement. Anywhere else the parens may be required:
/// as a command argument (`bits xor ((1..4) | each {..})`, including one on
/// its own line in a wrapped call), as a flag value (`--dir=((pwd) | ...)`),
/// as an operand (`((a) | b) + 1`), after a comparison (`$x == ((a) | b)`),
/// or after a `$x =` reassignment, which rejects a bare external command
/// such as `git` on its right-hand side.
fn is_droppable_pipeline_wrapper(line: &str, start: usize) -> bool {
    let Some(target) = line[..start].trim_end().strip_suffix('=') else {
        return false;
    };
    // Only the statement the wrapper belongs to: `a; let x = ((pwd) | ...)`.
    let target = target.rsplit(';').next().unwrap_or(target).trim();
    // Any other `=` means the wrapper follows an operator (`==`, `>=`) or a
    // flag value (`--dir=`), not the declaration itself.
    if target.contains('=') || !is_declaration_target(target) {
        return false;
    }

    let wrapper = line[start..].trim_end().as_bytes();
    parens_enclose(wrapper) && !has_code_byte(wrapper, b';')
}

/// Return `true` for exactly `let NAME`, `let NAME: type`, or the same with
/// `let-env`, `mut`, `const`, or `export const`.
fn is_declaration_target(target: &str) -> bool {
    let declaration = target
        .strip_prefix("export")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .map_or(target, |rest| {
            let rest = rest.trim_start();
            if rest.starts_with("const") { rest } else { "" }
        });
    let Some(rest) = ["let-env", "let", "mut", "const"]
        .iter()
        .find_map(|keyword| declaration.strip_prefix(keyword))
        .filter(|rest| rest.starts_with(char::is_whitespace))
    else {
        return false;
    };

    let rest = rest.trim_start();
    let name_len = rest
        .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '$')))
        .unwrap_or(rest.len());
    let after_name = rest[name_len..].trim_start();
    name_len > 0 && (after_name.is_empty() || after_name.starts_with(':'))
}

/// Attempt to insert missing commas between record fields.
fn try_repair_missing_record_commas(source: &str) -> (String, bool) {
    let insert_positions = detect_missing_record_comma_positions(source);

    if insert_positions.is_empty() {
        return (source.to_string(), false);
    }

    let mut repaired = String::with_capacity(source.len() + insert_positions.len() * 2);
    let mut next_insert_idx = 0;

    for (idx, ch) in source.char_indices() {
        while next_insert_idx < insert_positions.len() && insert_positions[next_insert_idx] == idx {
            repaired.push(',');
            repaired.push(' ');
            next_insert_idx += 1;
        }
        repaired.push(ch);
    }

    (repaired, true)
}

/// Attempt to simplify redundant `((head) | tail)` wrappers line by line.
fn try_repair_redundant_pipeline_subexpr(source: &str) -> (String, bool) {
    let mut output = String::with_capacity(source.len());
    let mut changed = false;
    let protected_ranges = find_protected_ranges(source);
    let mut line_start = 0;

    for line in source.split_inclusive('\n') {
        let line_offset = line_start;
        line_start += line.len();
        let (body, newline) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };

        let Some(start) = body
            .match_indices("((")
            .map(|(start, _)| start)
            .find(|&start| region_at(&protected_ranges, line_offset + start).is_none())
            .filter(|&start| is_droppable_pipeline_wrapper(body, start))
        else {
            output.push_str(line);
            continue;
        };

        let candidate = &body[start..];
        let candidate_trimmed = candidate.trim_end();
        if !(candidate_trimmed.contains('|') && candidate_trimmed.ends_with(')')) {
            output.push_str(line);
            continue;
        }

        let Some(inner) = candidate_trimmed.get(1..candidate_trimmed.len() - 1) else {
            output.push_str(line);
            continue;
        };
        let Some(pipe_idx) = inner.find('|') else {
            output.push_str(line);
            continue;
        };

        let left = inner[..pipe_idx].trim();
        let right = inner[pipe_idx + 1..].trim();
        if !parens_enclose(left.as_bytes()) {
            output.push_str(line);
            continue;
        }

        let Some(unwrapped_left) = left.get(1..left.len() - 1) else {
            output.push_str(line);
            continue;
        };
        if unwrapped_left.trim().is_empty() || right.is_empty() {
            output.push_str(line);
            continue;
        }

        output.push_str(&body[..start]);
        output.push_str(unwrapped_left.trim());
        output.push_str(" | ");
        output.push_str(right);
        output.push_str(newline);
        changed = true;
    }

    (output, changed)
}

// ─────────────────────────────────────────────────────────────────────────────
// Brace padding
// ─────────────────────────────────────────────────────────────────────────────

/// Add spaces inside braces that are jammed against content
/// (e.g. `{foo` → `{ foo`, `bar}` → `bar }`).
fn add_brace_padding_segment(segment: &str) -> (String, bool) {
    let bytes = segment.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len() + 8);
    let mut changed = false;

    for (idx, &byte) in bytes.iter().enumerate() {
        if byte == b'{' {
            output.push(byte);
            if let Some(next) = bytes.get(idx + 1)
                && !next.is_ascii_whitespace()
                && *next != b'}'
            {
                output.push(b' ');
                changed = true;
            }
            continue;
        }

        if byte == b'}' {
            if let Some(last) = output.last()
                && !last.is_ascii_whitespace()
                && *last != b'{'
            {
                output.push(b' ');
                changed = true;
            }
            output.push(byte);
            continue;
        }

        output.push(byte);
    }

    let output = String::from_utf8(output).unwrap_or_else(|_| segment.to_string());
    (output, changed)
}

/// Add brace padding, skipping string literal contents.
fn add_brace_padding_outside_strings(source: &str) -> (String, bool) {
    transform_outside_protected_ranges(source, add_brace_padding_segment)
}

// ─────────────────────────────────────────────────────────────────────────────
// Span merging and region repair orchestration
// ─────────────────────────────────────────────────────────────────────────────

/// Merge overlapping spans into a minimal set of non-overlapping ranges.
fn merge_spans(spans: &[Span], len: usize) -> Vec<Span> {
    let mut normalised: Vec<Span> = spans
        .iter()
        .filter_map(|span| {
            if span.start >= len || span.end <= span.start {
                return None;
            }
            Some(Span {
                start: span.start,
                end: span.end.min(len),
            })
        })
        .collect();

    normalised.sort_by_key(|span| (span.start, span.end));

    let mut merged: Vec<Span> = Vec::new();
    for span in normalised {
        if let Some(last) = merged.last_mut()
            && span.start <= last.end
        {
            last.end = last.end.max(span.end);
            continue;
        }
        merged.push(span);
    }

    merged
}

/// Grow the repair window `start..end` to whole lines that neither start
/// nor end inside a string literal or comment.
///
/// Each window is rescanned on its own. A window that opened mid-string
/// would mistake the string's closing quote for an opening one and expose
/// the string's contents to the repairs, and one that opened mid-word
/// could take the `#` of `nixpkgs#hello` for a comment. Whole lines also
/// let the line-based repairs see each line in full. Growing (rather than
/// shrinking) keeps a string's closing quote in the window, which the
/// missing-comma repair needs to see.
fn expand_repair_window(
    source: &[u8],
    mut start: usize,
    mut end: usize,
    protected_ranges: &[Region],
) -> (usize, usize) {
    start = start.min(source.len());
    end = end.clamp(start, source.len());
    loop {
        let mut new_start = source[..start]
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(0, |newline| newline + 1);
        if let Some(region) = region_at(protected_ranges, new_start) {
            new_start = region.start;
        }

        let last = if end > start { end - 1 } else { start };
        let mut new_end = source[last..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(source.len(), |newline| last + newline + 1);
        if let Some(region) = region_at(protected_ranges, new_end).filter(|r| r.start < new_end) {
            new_end = region.end;
        }

        if (new_start, new_end) == (start, end) {
            return (start, end);
        }
        start = new_start;
        end = new_end;
    }
}

/// Apply all repair strategies to a single source region.
///
/// Missing-comma repair runs only when `repair_record_commas` is set, because
/// a multiline record without commas is valid on its own.
fn repair_region(source: &str, repair_record_commas: bool) -> (String, bool) {
    let (repaired_if_else, if_else_changed) = try_repair_compact_if_else(source);
    let (mut repaired_record, record_changed) = if repair_record_commas {
        try_repair_missing_record_commas(&repaired_if_else)
    } else {
        (repaired_if_else, false)
    };
    let (repaired_pipeline, pipeline_changed) =
        try_repair_redundant_pipeline_subexpr(&repaired_record);
    repaired_record = repaired_pipeline;

    let mut brace_spacing_changed = false;
    if record_changed {
        let (padded, padded_changed) = add_brace_padding_outside_strings(&repaired_record);
        repaired_record = padded;
        brace_spacing_changed = padded_changed;
    }

    (
        repaired_record,
        if_else_changed || record_changed || pipeline_changed || brace_spacing_changed,
    )
}

/// Parse error spans, ready to answer "does this span touch an error?"
/// in logarithmic time.
pub(super) struct ErrorSpans {
    /// The spans, sorted by start.
    spans: Vec<Span>,
    /// `max_end[i]` is the largest end among `spans[..=i]`.
    max_end: Vec<usize>,
}

impl ErrorSpans {
    pub(super) fn new(spans: &[Span]) -> Self {
        let mut spans = spans.to_vec();
        spans.sort_by_key(|span| span.start);
        let max_end = spans
            .iter()
            .scan(0, |max_end, span| {
                *max_end = span.end.max(*max_end);
                Some(*max_end)
            })
            .collect();
        Self { spans, max_end }
    }

    /// Return `true` when `span` touches any error. Touching counts, since
    /// some parse errors have an empty span.
    pub(super) fn touches(&self, span: Span) -> bool {
        let candidates = self.spans.partition_point(|error| error.start <= span.end);
        candidates > 0 && self.max_end[candidates - 1] >= span.start
    }
}

/// Attempt to repair parse errors by patching malformed source regions.
///
/// Missing record commas are inserted only in regions that touch one of
/// `parse_error_spans`. Source that is not valid UTF-8 is left alone, so
/// its bytes are never replaced.
///
/// Returns `Some(ParseRepairOutcome::Reformat(…))` with the patched source
/// bytes if any repair was applied, or `None` if nothing could be done.
pub(super) fn try_repair_parse_errors(
    contents: &[u8],
    malformed_spans: &[Span],
    parse_error_spans: &[Span],
) -> Option<ParseRepairOutcome> {
    if malformed_spans.is_empty() {
        return None;
    }

    let source = std::str::from_utf8(contents).ok()?;
    let protected_ranges = find_protected_ranges(source);
    let windows: Vec<Span> = malformed_spans
        .iter()
        .map(|span| {
            let (start, end) =
                expand_repair_window(contents, span.start, span.end, &protected_ranges);
            Span { start, end }
        })
        .collect();
    let spans = merge_spans(&windows, source.len());

    if spans.is_empty() {
        return None;
    }

    let errors = ErrorSpans::new(parse_error_spans);
    let mut cursor = 0;
    let mut output = String::with_capacity(source.len());
    let mut changed = false;

    for Span { start, end } in spans {
        if start >= end || start < cursor {
            continue;
        }

        output.push_str(&source[cursor..start]);

        let repair_record_commas = errors.touches(Span::new(start, end));
        let (repaired_region, region_changed) =
            repair_region(&source[start..end], repair_record_commas);
        output.push_str(&repaired_region);
        changed |= region_changed;

        cursor = end;
    }

    output.push_str(&source[cursor..]);

    if changed {
        Some(ParseRepairOutcome::Reformat(output.into_bytes()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_record_comma_ignores_text_inside_interpolations_issue220() {
        assert!(detect_missing_record_comma_positions(r#"$"("a:")""#).is_empty());
        assert!(detect_missing_record_comma_positions(r#"$"(f "x" a: 1)""#).is_empty());
        assert_eq!(
            detect_missing_record_comma_positions(r#"{a: $"("b:")"c: 1}"#),
            [13]
        );
    }

    #[test]
    fn missing_record_comma_after_paren_value() {
        assert_eq!(detect_missing_record_comma_positions("{a: (f)b: 1}"), [7]);
        assert!(detect_missing_record_comma_positions("{a: (f) # b: 1\n}").is_empty());
    }

    #[test]
    fn missing_record_comma_needs_a_key_glued_to_the_value() {
        // Fields separated by whitespace are valid without commas.
        assert!(detect_missing_record_comma_positions("{a: (f) b: 1}").is_empty());
        assert!(detect_missing_record_comma_positions("{a: \"x\"\n b: 1}").is_empty());
        assert!(detect_missing_record_comma_positions("print \"x\" https://a.b").is_empty());
        assert_eq!(
            detect_missing_record_comma_positions("{name:\"Alice\"age:30}"),
            [13]
        );
    }

    #[test]
    fn engine_gap_errors_are_not_syntax_errors() {
        let syntax = |error: &str| {
            is_syntax_error(&ParseError::LabeledErrorWithHelp {
                error: error.into(),
                label: String::new(),
                help: String::new(),
                span: Span::test_data(),
            })
        };
        assert!(syntax("Unexpected token in record"));
        assert!(syntax("Unexpected semicolon in list"));
        assert!(!syntax(
            "External command calls must be explicit in assignments"
        ));
        assert!(!syntax("percent sigil requires a built-in command"));
    }

    #[test]
    fn pipeline_wrapper_droppable_only_after_declaration() {
        let droppable = |line: &str| is_droppable_pipeline_wrapper(line, line.find("((").unwrap());

        assert!(droppable("let p = ((pwd) | path join x)"));
        assert!(droppable("let x: string = ((pwd) | str trim)"));
        assert!(droppable("export const x = ((pwd) | str trim)"));
        assert!(droppable("let a = 1; let x = ((pwd) | str trim)"));

        assert!(droppable("let $n = ((pwd) | str trim)"));
        assert!(droppable("let x: list<string> = ((pwd) | lines)"));

        // A reassignment rejects a bare external command on its right-hand
        // side (`$x = git ...`), so its parens stay.
        assert!(!droppable("$x = ((git rev-parse HEAD) | str trim)"));
        assert!(!droppable("$env.X = ((pwd) | str trim)"));

        // Dropping the parens would split the wrapper into two statements.
        assert!(!droppable("let x = ((print 1; 5) | $in + 1)"));
        assert!(!droppable("let x = ((pwd) | str trim; ls)"));
        assert!(droppable("let x = ((pwd) | str replace \";\" \",\")"));

        // After a declaration, a later `=` belongs to an operator or flag.
        assert!(!droppable("let ok = $x == ((pwd) | str length)"));
        assert!(!droppable("let ok = $x >= ((pwd) | str length)"));
        assert!(!droppable("let x = ls --dir=((pwd) | path join x)"));
        assert!(!droppable("let r = $r | upsert b=((pwd) | str trim)"));
        assert!(!droppable("let x = foo ((pwd) | str trim)"));
        assert!(!droppable("export def x = ((pwd) | str trim)"));

        // A line can start with `((` as an argument of a wrapped call.
        assert!(!droppable("    ((rich render x) | str contains y)"));
        assert!(!droppable("f --dir=((pwd) | path join x)"));
        assert!(!droppable("$r | upsert b=((pwd) | str trim)"));
        assert!(!droppable("$x+=((pwd) | str length)"));
        assert!(!droppable("$key | bits xor ((1..4) | each {0x[36]})"));
        assert!(!droppable("let s = 1.0 - ((2.0 * $l) | math abs)"));
        assert!(!droppable("if $x == ((pwd) | str length) { 1 }"));
        assert!(!droppable("$x += ((pwd) | str length)"));
        assert!(!droppable("let x = ((a) | b) + ((c) | d)"));
    }

    #[test]
    fn repair_window_grows_to_whole_lines_outside_strings_and_comments() {
        fn window(source: &str, start: usize, end: usize) -> &str {
            let regions = scan_regions(source.as_bytes());
            let (start, end) = expand_repair_window(source.as_bytes(), start, end, &regions);
            &source[start..end]
        }

        let source = "a\nlet x = 1 + 2\nb";
        assert_eq!(window(source, 8, 9), "let x = 1 + 2\n");
        assert_eq!(window(source, 8, 8), "let x = 1 + 2\n");

        // A string that spans lines is taken whole, with the lines around it.
        let source = "x\nlet s = \"one\ntwo\" | f\ny";
        assert_eq!(window(source, 15, 16), "let s = \"one\ntwo\" | f\n");

        // `nixpkgs#hello` is a word, so the window starts at its line.
        let source = "^nix run nixpkgs#hello \"a\nb\"\nlet y = 1";
        assert_eq!(window(source, 18, 19), "^nix run nixpkgs#hello \"a\nb\"\n");
    }

    #[test]
    fn redundant_pipeline_repair_sees_whole_lines() {
        // The second line's window used to end right after `str trim)`, so
        // the repair dropped the parens that the `+ "a"` needs.
        let source =
            "let p = ((pwd) | path join x)\nlet yyyyy = ((pwd) | str trim) + \"a\"\nprint $yyyyy";
        let malformed = detect_redundant_pipeline_subexpr_spans(source);
        let Some(ParseRepairOutcome::Reformat(repaired)) =
            try_repair_parse_errors(source.as_bytes(), &malformed, &[])
        else {
            panic!("expected a repair");
        };
        assert_eq!(
            String::from_utf8(repaired).unwrap(),
            "let p = pwd | path join x\nlet yyyyy = ((pwd) | str trim) + \"a\"\nprint $yyyyy"
        );
    }

    #[test]
    fn repairs_leave_invalid_utf8_alone() {
        let source = b"let s = \"caf\xe9\"\nlet x = ((pwd) | str trim)";
        let malformed = [Span::new(18, 44)];
        assert!(try_repair_parse_errors(source, &malformed, &[]).is_none());
    }

    #[test]
    fn compact_if_else_repair_skips_strings_after_a_word_with_a_hash() {
        let source = "^nix run nixpkgs#hello \"hi\nthere\"\nlet js = \"if(x){y}else{z}\"";
        assert!(detect_compact_if_else_spans(source).is_empty());
    }

    #[test]
    fn error_spans_touch_overlapping_and_adjacent_spans() {
        let errors = ErrorSpans::new(&[Span::new(30, 40), Span::new(5, 50), Span::new(60, 60)]);
        assert!(errors.touches(Span::new(0, 5)));
        assert!(errors.touches(Span::new(45, 55)));
        assert!(errors.touches(Span::new(60, 70)));
        assert!(!errors.touches(Span::new(51, 59)));
        assert!(!errors.touches(Span::new(61, 70)));
        assert!(!ErrorSpans::new(&[]).touches(Span::new(0, 10)));
    }

    #[test]
    fn missing_comma_after_long_string_is_repaired() {
        let description = "a very long description that is more than thirty two bytes";
        let source = format!("let user = {{description: \"{description}\"other: 1}}");
        let error_at = source.find("other").unwrap();
        let parse_errors = [Span::new(error_at, error_at + 5)];
        // As in `format_inner`: the error plus the detected comma windows,
        // whose start lands inside the long string.
        let mut malformed = parse_errors.to_vec();
        malformed.extend(detect_missing_record_comma_spans(&source));
        let Some(ParseRepairOutcome::Reformat(repaired)) =
            try_repair_parse_errors(source.as_bytes(), &malformed, &parse_errors)
        else {
            panic!("expected a repair");
        };
        let repaired = String::from_utf8(repaired).unwrap();
        assert!(
            repaired.contains(&format!("\"{description}\", other: 1")),
            "{repaired}"
        );
    }

    #[test]
    fn redundant_pipeline_repair_skips_multiline_strings() {
        let source = "let s = \"\nlet x = ((pwd) | str trim)\n\"";
        assert_eq!(
            try_repair_redundant_pipeline_subexpr(source),
            (source.to_string(), false)
        );
    }
}
