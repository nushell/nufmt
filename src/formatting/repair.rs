//! Parse-error repair utilities.
//!
//! When the Nushell parser emits recoverable errors (compact `if/else`,
//! missing record commas, etc.), these routines attempt to patch the
//! source text so a second parse succeeds cleanly. Repairs are
//! region-scoped and never mutate string-literal contents.

use nu_protocol::{ParseError, Span};

/// Whether a parse error is fatal enough that formatting should be skipped.
pub(super) fn is_fatal_parse_error(error: &ParseError) -> bool {
    matches!(
        error,
        ParseError::ExtraTokens(_)
            | ParseError::ExtraTokensAfterClosingDelimiter(_)
            | ParseError::UnexpectedEof(_, _)
            | ParseError::Unclosed(_, _)
            | ParseError::Unbalanced(_, _, _)
            | ParseError::IncompleteMathExpression(_)
            | ParseError::UnknownCommand(_)
            | ParseError::Expected(_, _)
            | ParseError::ExpectedWithStringMsg(_, _)
            | ParseError::ExpectedWithDidYouMean(_, _, _)
    )
}

/// The outcome of a successful repair pass.
pub(super) enum ParseRepairOutcome {
    /// Repaired source bytes ready for re-parsing and re-formatting.
    Reformat(Vec<u8>),
}

// ─────────────────────────────────────────────────────────────────────────────
// String-literal safeguards
// ─────────────────────────────────────────────────────────────────────────────

/// Find byte ranges of string literals (single- and double-quoted) so that
/// transformations can skip their contents.
fn find_string_literal_ranges(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut ranges = Vec::new();
    let mut in_string: Option<u8> = None;
    let mut escaped = false;
    let mut start = 0;

    for (idx, &byte) in bytes.iter().enumerate() {
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                continue;
            }
            if byte == quote {
                ranges.push((start, idx + 1));
                in_string = None;
            }
            continue;
        }

        if byte == b'"' || byte == b'\'' {
            start = idx;
            in_string = Some(byte);
            escaped = false;
        }
    }

    // Unclosed string — extend to EOF
    if in_string.is_some() {
        ranges.push((start, source.len()));
    }

    ranges
}

/// Apply a transformation only to the non-string-literal portions of `source`.
fn transform_outside_string_literals(
    source: &str,
    mut transform: impl FnMut(&str) -> (String, bool),
) -> (String, bool) {
    let string_ranges = find_string_literal_ranges(source);

    if string_ranges.is_empty() {
        return transform(source);
    }

    let mut output = String::with_capacity(source.len());
    let mut changed = false;
    let mut cursor = 0;

    for (start, end) in string_ranges {
        if cursor < start {
            let (transformed, seg_changed) = transform(&source[cursor..start]);
            output.push_str(&transformed);
            changed |= seg_changed;
        }
        output.push_str(&source[start..end]);
        cursor = end;
    }

    if cursor < source.len() {
        let (transformed, seg_changed) = transform(&source[cursor..]);
        output.push_str(&transformed);
        changed |= seg_changed;
    }

    (output, changed)
}

/// Return ranges of `source` that are *not* inside string literals.
fn non_string_ranges(source: &str) -> Vec<(usize, usize)> {
    let string_ranges = find_string_literal_ranges(source);

    if string_ranges.is_empty() {
        return vec![(0, source.len())];
    }

    let mut ranges = Vec::new();
    let mut cursor = 0;

    for (start, end) in string_ranges {
        if cursor < start {
            ranges.push((cursor, start));
        }
        cursor = end;
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

    for (range_start, range_end) in non_string_ranges(source) {
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
    transform_outside_string_literals(source, repair_compact_if_else_segment)
}

// ─────────────────────────────────────────────────────────────────────────────
// Missing record-comma repair
// ─────────────────────────────────────────────────────────────────────────────

/// Detect byte positions where a comma is likely missing between record fields.
fn detect_missing_record_comma_positions(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let mut in_string = false;
    let mut escaped = false;
    let mut insert_positions: Vec<usize> = Vec::new();

    for (idx, &byte) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                continue;
            }
            if byte == b'"' {
                in_string = false;

                // Look ahead for `identifier:` pattern (next record key)
                let mut lookahead = idx + 1;
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                    lookahead += 1;
                }

                if lookahead < bytes.len()
                    && (bytes[lookahead].is_ascii_alphabetic() || bytes[lookahead] == b'_')
                {
                    let mut key_end = lookahead;
                    while key_end < bytes.len()
                        && (bytes[key_end].is_ascii_alphanumeric()
                            || bytes[key_end] == b'_'
                            || bytes[key_end] == b'-')
                    {
                        key_end += 1;
                    }

                    if key_end < bytes.len() && bytes[key_end] == b':' {
                        let between = &bytes[idx + 1..lookahead];
                        if !between.contains(&b',') {
                            insert_positions.push(idx + 1);
                        }
                    }
                }
            }
            continue;
        }

        if byte == b'"' {
            in_string = true;
            escaped = false;
        }
    }

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

    for (range_start, range_end) in non_string_ranges(source) {
        let segment = &source[range_start..range_end];
        for (offset, _) in segment.match_indices("((") {
            let idx = range_start + offset;
            let line_end = source[idx..]
                .find('\n')
                .map_or(source.len(), |rel| idx + rel);
            let line = &source[idx..line_end];
            if line.contains('|') && line.trim_end().ends_with(')') {
                spans.push(Span {
                    start: idx.saturating_sub(32),
                    end: (line_end + 32).min(source.len()),
                });
            }
        }
    }

    spans
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

    for line in source.split_inclusive('\n') {
        let (body, newline) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };

        let Some(start) = body.find("((") else {
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
        if !(left.starts_with('(') && left.ends_with(')')) {
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
            if let Some(next) = bytes.get(idx + 1) {
                if !next.is_ascii_whitespace() && *next != b'}' {
                    output.push(b' ');
                    changed = true;
                }
            }
            continue;
        }

        if byte == b'}' {
            if let Some(last) = output.last() {
                if !last.is_ascii_whitespace() && *last != b'{' {
                    output.push(b' ');
                    changed = true;
                }
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
    transform_outside_string_literals(source, add_brace_padding_segment)
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
        if let Some(last) = merged.last_mut() {
            if span.start <= last.end {
                last.end = last.end.max(span.end);
                continue;
            }
        }
        merged.push(span);
    }

    merged
}

/// Adjust an index to the nearest valid UTF-8 char boundary.
fn align_to_char_boundary(source: &str, index: usize, forward: bool) -> usize {
    let mut adjusted = index.min(source.len());

    if forward {
        while adjusted < source.len() && !source.is_char_boundary(adjusted) {
            adjusted += 1;
        }
    } else {
        while adjusted > 0 && !source.is_char_boundary(adjusted) {
            adjusted -= 1;
        }
    }

    adjusted
}

/// Apply all repair strategies to a single source region.
fn repair_region(source: &str) -> (String, bool) {
    let (repaired_if_else, if_else_changed) = try_repair_compact_if_else(source);
    let (mut repaired_record, record_changed) = try_repair_missing_record_commas(&repaired_if_else);
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

/// Attempt to repair parse errors by patching malformed source regions.
///
/// Returns `Some(ParseRepairOutcome::Reformat(…))` with the patched source
/// bytes if any repair was applied, or `None` if nothing could be done.
pub(super) fn try_repair_parse_errors(
    contents: &[u8],
    malformed_spans: &[Span],
) -> Option<ParseRepairOutcome> {
    if malformed_spans.is_empty() {
        return None;
    }

    let source = String::from_utf8_lossy(contents).into_owned();
    let spans = merge_spans(malformed_spans, source.len());

    if spans.is_empty() {
        return None;
    }

    let mut cursor = 0;
    let mut output = String::with_capacity(source.len());
    let mut changed = false;

    for span in spans {
        let start = align_to_char_boundary(&source, span.start, false);
        let end = align_to_char_boundary(&source, span.end, true);

        if start >= end || start < cursor {
            continue;
        }

        output.push_str(&source[cursor..start]);

        let (repaired_region, region_changed) = repair_region(&source[start..end]);
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
