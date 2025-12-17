//! In this module occurs most of the magic in `nufmt`.
//!
//! It walks the Nushell AST and emits properly formatted code.

use crate::config::Config;
use crate::format_error::FormatError;
use log::{debug, trace};
use nu_parser::parse;
use nu_protocol::{
    ast::{
        Argument, Block, Expr, Expression, ExternalArgument, ListItem, MatchPattern, PathMember,
        Pattern, Pipeline, PipelineElement, PipelineRedirection, RecordItem, RedirectionTarget,
    },
    engine::{EngineState, StateWorkingSet},
    Span,
};

/// Get the default engine state with built-in commands
fn get_engine_state() -> EngineState {
    nu_cmd_lang::create_default_context()
}

/// The main formatter context that tracks indentation and other state
struct Formatter<'a> {
    /// The original source bytes
    source: &'a [u8],
    /// The working set for looking up blocks and other data
    working_set: &'a StateWorkingSet<'a>,
    /// Configuration options
    config: &'a Config,
    /// Current indentation level
    indent_level: usize,
    /// Output buffer
    output: Vec<u8>,
    /// Track if we're at the start of a line (for indentation)
    at_line_start: bool,
    /// Comments extracted from source, indexed by their end position
    comments: Vec<(Span, Vec<u8>)>,
    /// Track which comments have been written
    written_comments: Vec<bool>,
    /// Current position in source being processed
    last_pos: usize,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a [u8], working_set: &'a StateWorkingSet<'a>, config: &'a Config) -> Self {
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
        }
    }

    /// Write indentation if at start of line
    fn write_indent(&mut self) {
        if self.at_line_start {
            let indent = " ".repeat(self.config.indent * self.indent_level);
            self.output.extend(indent.as_bytes());
            self.at_line_start = false;
        }
    }

    /// Write a string to output
    fn write(&mut self, s: &str) {
        self.write_indent();
        self.output.extend(s.as_bytes());
    }

    /// Write bytes to output
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_indent();
        self.output.extend(bytes);
    }

    /// Write a newline
    fn newline(&mut self) {
        self.output.push(b'\n');
        self.at_line_start = true;
    }

    /// Write a space if not at line start
    fn space(&mut self) {
        if !self.at_line_start && !self.output.is_empty() {
            let last = *self.output.last().unwrap();
            if last != b' ' && last != b'\n' && last != b'\t' && last != b'(' && last != b'[' {
                self.output.push(b' ');
            }
        }
    }

    /// Get the source content for a span
    fn get_span_content(&self, span: Span) -> Vec<u8> {
        self.source[span.start..span.end].to_vec()
    }

    /// Check if there are any comments between last_pos and the given position
    fn write_comments_before(&mut self, pos: usize) {
        let mut comments_to_write = Vec::new();
        for (i, (span, content)) in self.comments.iter().enumerate() {
            if !self.written_comments[i] && span.start >= self.last_pos && span.end <= pos {
                comments_to_write.push((i, span.start, content.clone()));
            }
        }
        comments_to_write.sort_by_key(|(_, start, _)| *start);

        for (idx, _, content) in comments_to_write {
            self.written_comments[idx] = true;
            // Check if we need a newline before the comment
            if !self.at_line_start && !self.output.is_empty() {
                let last = *self.output.last().unwrap();
                if last != b'\n' {
                    self.newline();
                }
            }
            self.write_indent();
            self.output.extend(&content);
            self.newline();
        }
    }

    /// Check for inline comment after a position (on the same line)
    fn write_inline_comment(&mut self, after_pos: usize) {
        // Look for a comment that starts on the same line as after_pos
        let line_end = self.source[after_pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| after_pos + p)
            .unwrap_or(self.source.len());

        let mut found_comment: Option<(usize, Span, Vec<u8>)> = None;
        for (i, (span, content)) in self.comments.iter().enumerate() {
            if !self.written_comments[i] && span.start >= after_pos && span.start < line_end {
                found_comment = Some((i, *span, content.clone()));
                break;
            }
        }

        if let Some((idx, span, content)) = found_comment {
            self.written_comments[idx] = true;
            self.write(" ");
            self.output.extend(&content);
            self.last_pos = span.end;
        }
    }

    /// Format a block
    fn format_block(&mut self, block: &Block) {
        let num_pipelines = block.pipelines.len();
        for (i, pipeline) in block.pipelines.iter().enumerate() {
            // Write any comments before this pipeline
            if let Some(first_elem) = pipeline.elements.first() {
                self.write_comments_before(first_elem.expr.span.start);
            }

            self.format_pipeline(pipeline);

            // Check for inline comments after the pipeline
            if let Some(last_elem) = pipeline.elements.last() {
                let end_pos = if let Some(ref redir) = last_elem.redirection {
                    match redir {
                        PipelineRedirection::Single { target, .. } => target.span().end,
                        PipelineRedirection::Separate { out, err } => {
                            out.span().end.max(err.span().end)
                        }
                    }
                } else {
                    last_elem.expr.span.end
                };
                self.write_inline_comment(end_pos);
                self.last_pos = end_pos;
            }

            if i < num_pipelines - 1 {
                self.newline();
            }
        }
    }

    /// Format a pipeline
    fn format_pipeline(&mut self, pipeline: &Pipeline) {
        for (i, element) in pipeline.elements.iter().enumerate() {
            if i > 0 {
                // Pipe between elements - space before and after
                self.write(" | ");
            }
            self.format_pipeline_element(element);
        }
    }

    /// Format a pipeline element
    fn format_pipeline_element(&mut self, element: &PipelineElement) {
        self.format_expression(&element.expr);

        // Handle redirections
        if let Some(ref redirection) = element.redirection {
            self.format_redirection(redirection);
        }
    }

    /// Format a redirection
    fn format_redirection(&mut self, redir: &PipelineRedirection) {
        match redir {
            PipelineRedirection::Single { target, .. } => {
                self.space();
                self.format_redirection_target(target);
            }
            PipelineRedirection::Separate { out, err } => {
                self.space();
                self.format_redirection_target(out);
                self.space();
                self.format_redirection_target(err);
            }
        }
    }

    /// Format a redirection target
    fn format_redirection_target(&mut self, target: &RedirectionTarget) {
        match target {
            RedirectionTarget::File { expr, span, .. } => {
                let redir_content = self.get_span_content(*span);
                self.write_bytes(&redir_content);
                self.space();
                self.format_expression(expr);
            }
            RedirectionTarget::Pipe { span } => {
                let content = self.get_span_content(*span);
                self.write_bytes(&content);
            }
        }
    }

    /// Format an expression
    fn format_expression(&mut self, expr: &Expression) {
        match &expr.expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) | Expr::Nothing | Expr::DateTime(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::String(_) | Expr::RawString(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Binary(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Filepath(_, _) | Expr::Directory(_, _) | Expr::GlobPattern(_, _) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Var(_) | Expr::VarDecl(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Call(call) => {
                // Get the command name
                let decl = self.working_set.get_decl(call.decl_id);
                let decl_name = decl.name();

                // Check if this is a special keyword-based command
                let is_def =
                    decl_name == "def" || decl_name == "def-env" || decl_name == "export def";
                let is_if = decl_name == "if";
                let is_let = decl_name == "let"
                    || decl_name == "let-env"
                    || decl_name == "mut"
                    || decl_name == "const";
                let is_try = decl_name == "try";
                let is_for = decl_name == "for";
                let is_while = decl_name == "while";
                let is_loop = decl_name == "loop";
                let is_module = decl_name == "module";

                // Write command name
                if call.head.end != 0 {
                    let head_content = self.get_span_content(call.head);
                    self.write_bytes(&head_content);
                }

                // Format arguments
                for arg in &call.arguments {
                    match arg {
                        Argument::Positional(positional) | Argument::Unknown(positional) => {
                            // Handle special cases for def signatures and blocks
                            if is_def {
                                match &positional.expr {
                                    Expr::String(_) => {
                                        // Function name
                                        self.space();
                                        self.format_expression(positional);
                                    }
                                    Expr::Signature(_) => {
                                        // Signature - format specially
                                        self.space();
                                        self.format_signature_expression(positional);
                                    }
                                    Expr::Closure(block_id) | Expr::Block(block_id) => {
                                        // Function body
                                        self.space();
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.space();
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_if || is_try {
                                match &positional.expr {
                                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                                        self.space();
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.space();
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_let {
                                self.space();
                                // For let/mut/const, we need to handle VarDecl and the value specially
                                match &positional.expr {
                                    Expr::VarDecl(_) => {
                                        self.format_expression(positional);
                                    }
                                    Expr::Block(block_id) => {
                                        // The value is wrapped in a block for let statements
                                        // Output the = sign before the value
                                        self.write("= ");
                                        let block = self.working_set.get_block(*block_id);
                                        // Format the block contents inline
                                        self.format_block(block);
                                    }
                                    _ => {
                                        self.write("= ");
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_for {
                                // for loop: `for x in list { body }`
                                self.space();
                                match &positional.expr {
                                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_while {
                                // while loop: `while condition { body }`
                                self.space();
                                match &positional.expr {
                                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_loop {
                                // loop: `loop { body }`
                                self.space();
                                match &positional.expr {
                                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.format_expression(positional);
                                    }
                                }
                            } else if is_module {
                                // module: `module name { body }`
                                self.space();
                                match &positional.expr {
                                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                                        self.format_block_expression(
                                            *block_id,
                                            positional.span,
                                            true,
                                        );
                                    }
                                    _ => {
                                        self.format_expression(positional);
                                    }
                                }
                            } else {
                                // Regular command argument
                                self.space();
                                self.format_expression(positional);
                            }
                        }
                        Argument::Named(named) => {
                            self.space();
                            // Write the flag
                            if named.0.span.end != 0 {
                                let flag_content = self.get_span_content(named.0.span);
                                self.write_bytes(&flag_content);
                            }
                            // Write the short flag if present
                            if let Some(short) = &named.1 {
                                let short_content = self.get_span_content(short.span);
                                self.write_bytes(&short_content);
                            }
                            // Write the value if present
                            if let Some(value) = &named.2 {
                                self.space();
                                self.format_expression(value);
                            }
                        }
                        Argument::Spread(spread_expr) => {
                            self.space();
                            self.write("...");
                            self.format_expression(spread_expr);
                        }
                    }
                }
            }

            Expr::ExternalCall(head, args) => {
                // Format external command head
                self.format_expression(head);

                // Format arguments
                for arg in args.as_ref() {
                    self.space();
                    match arg {
                        ExternalArgument::Regular(arg_expr) => {
                            self.format_expression(arg_expr);
                        }
                        ExternalArgument::Spread(spread_expr) => {
                            self.write("...");
                            self.format_expression(spread_expr);
                        }
                    }
                }
            }

            Expr::Operator(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::BinaryOp(lhs, op, rhs) => {
                self.format_expression(lhs);
                self.space();
                self.format_expression(op);
                self.space();
                self.format_expression(rhs);
            }

            Expr::UnaryNot(inner) => {
                self.write("not ");
                self.format_expression(inner);
            }

            Expr::Block(block_id) => {
                self.format_block_expression(*block_id, expr.span, false);
            }

            Expr::Closure(block_id) => {
                self.format_closure_expression(*block_id, expr.span);
            }

            Expr::Subexpression(block_id) => {
                self.write("(");
                let block = self.working_set.get_block(*block_id);
                // Format inline if simple
                if block.pipelines.len() == 1 && block.pipelines[0].elements.len() <= 3 {
                    self.format_block(block);
                } else {
                    self.newline();
                    self.indent_level += 1;
                    self.format_block(block);
                    self.newline();
                    self.indent_level -= 1;
                    self.write_indent();
                }
                self.write(")");
            }

            Expr::List(items) => {
                self.format_list(items, expr.span);
            }

            Expr::Record(items) => {
                self.format_record(items, expr.span);
            }

            Expr::Table(table) => {
                self.format_table(&table.columns, &table.rows, expr.span);
            }

            Expr::Range(range) => {
                if let Some(from) = &range.from {
                    self.format_expression(from);
                }
                if let Some(next) = &range.next {
                    self.write(",");
                    self.format_expression(next);
                }
                let op_content = self.get_span_content(range.operator.span);
                self.write_bytes(&op_content);
                if let Some(to) = &range.to {
                    self.format_expression(to);
                }
            }

            Expr::CellPath(cell_path) => {
                for member in &cell_path.members {
                    match member {
                        PathMember::String { val, optional, .. } => {
                            self.write(".");
                            if *optional {
                                self.write("?");
                            }
                            self.write(val);
                        }
                        PathMember::Int { val, optional, .. } => {
                            self.write(".");
                            if *optional {
                                self.write("?");
                            }
                            self.write(&val.to_string());
                        }
                    }
                }
            }

            Expr::FullCellPath(full_path) => {
                self.format_expression(&full_path.head);
                for member in &full_path.tail {
                    match member {
                        PathMember::String { val, optional, .. } => {
                            self.write(".");
                            if *optional {
                                self.write("?");
                            }
                            self.write(val);
                        }
                        PathMember::Int { val, optional, .. } => {
                            self.write(".");
                            if *optional {
                                self.write("?");
                            }
                            self.write(&val.to_string());
                        }
                    }
                }
            }

            Expr::StringInterpolation(_) => {
                // Use original content for string interpolation to preserve structure
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::GlobInterpolation(_, _) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::RowCondition(block_id) => {
                // Row conditions are usually simple expressions
                let block = self.working_set.get_block(*block_id);
                self.format_block(block);
            }

            Expr::Keyword(keyword) => {
                let kw_content = self.get_span_content(keyword.span);
                self.write_bytes(&kw_content);
                self.space();
                // Handle the expression after the keyword (e.g., else block)
                match &keyword.expr.expr {
                    Expr::Block(block_id) | Expr::Closure(block_id) => {
                        self.format_block_expression(*block_id, keyword.expr.span, true);
                    }
                    _ => {
                        self.format_expression(&keyword.expr);
                    }
                }
            }

            Expr::ValueWithUnit(value_unit) => {
                self.format_expression(&value_unit.expr);
                let unit_content = self.get_span_content(value_unit.unit.span);
                self.write_bytes(&unit_content);
            }

            Expr::MatchBlock(matches) => {
                self.format_match_block(matches);
            }

            Expr::Signature(_) => {
                // Format signature
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::ImportPattern(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Overlay(_) => {
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }

            Expr::Collect(_, inner) => {
                self.format_expression(inner);
            }

            Expr::AttributeBlock(attr_block) => {
                for attr in &attr_block.attributes {
                    let content = self.get_span_content(attr.expr.span);
                    self.write_bytes(&content);
                    self.newline();
                }
                self.format_expression(&attr_block.item);
            }

            Expr::Garbage => {
                // Output original garbage content
                let content = self.get_span_content(expr.span);
                self.write_bytes(&content);
            }
        }
    }

    /// Format a signature expression (for def commands)
    fn format_signature_expression(&mut self, expr: &Expression) {
        let content = self.get_span_content(expr.span);
        // Parse and reformat the signature to ensure consistent spacing
        self.write_bytes(&content);
    }

    /// Format a block expression with braces
    fn format_block_expression(
        &mut self,
        block_id: nu_protocol::BlockId,
        _span: Span,
        with_braces: bool,
    ) {
        let block = self.working_set.get_block(block_id);

        if with_braces {
            self.write("{");
        }

        // Check if block is simple enough to be inline
        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block);

        if is_simple && with_braces {
            self.write(" ");
            self.format_block(block);
            self.write(" ");
        } else if block.pipelines.is_empty() {
            // Empty block
            if with_braces {
                self.write(" ");
            }
        } else {
            self.newline();
            self.indent_level += 1;
            self.format_block(block);
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }

        if with_braces {
            self.write("}");
        }
    }

    /// Check if a block has nested structures that require multiline formatting
    fn block_has_nested_structures(&self, block: &Block) -> bool {
        for pipeline in &block.pipelines {
            for element in &pipeline.elements {
                if self.expr_is_complex(&element.expr) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if an expression is complex enough to warrant multiline formatting
    fn expr_is_complex(&self, expr: &Expression) -> bool {
        match &expr.expr {
            Expr::Block(_) | Expr::Closure(_) => true,
            Expr::List(items) => items.len() > 3,
            Expr::Record(items) => items.len() > 2,
            Expr::Call(call) => call.arguments.iter().any(|arg| match arg {
                Argument::Positional(e) | Argument::Unknown(e) | Argument::Spread(e) => {
                    self.expr_is_complex(e)
                }
                Argument::Named(n) => n.2.as_ref().is_some_and(|e| self.expr_is_complex(e)),
            }),
            _ => false,
        }
    }

    /// Format a closure expression
    fn format_closure_expression(&mut self, block_id: nu_protocol::BlockId, span: Span) {
        let content = self.get_span_content(span);
        // Check if this closure has parameters (starts with {|)
        let has_params = content.starts_with(b"{|") || content.starts_with(b"{ |");

        if has_params {
            // Find the end of the parameter section
            let param_end = content.iter().position(|&b| b == b'|').and_then(|first| {
                content[first + 1..]
                    .iter()
                    .position(|&b| b == b'|')
                    .map(|p| first + 1 + p + 1)
            });

            if let Some(end) = param_end {
                self.write("{|");
                // Extract parameter content (between the two |)
                let params = &content[2..end - 1];
                let trimmed = params
                    .iter()
                    .copied()
                    .skip_while(|b| b.is_ascii_whitespace())
                    .collect::<Vec<_>>();
                let trimmed: Vec<u8> = trimmed
                    .into_iter()
                    .rev()
                    .skip_while(|b| b.is_ascii_whitespace())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                self.write_bytes(&trimmed);
                self.write("| ");

                // Format the body
                let block = self.working_set.get_block(block_id);
                let is_simple = block.pipelines.len() == 1
                    && block.pipelines[0].elements.len() == 1
                    && !self.block_has_nested_structures(block);

                if is_simple {
                    self.format_block(block);
                    self.write(" }");
                } else {
                    self.newline();
                    self.indent_level += 1;
                    self.format_block(block);
                    self.newline();
                    self.indent_level -= 1;
                    self.write_indent();
                    self.write("}");
                }
            } else {
                // Fallback: just output original
                self.write_bytes(&content);
            }
        } else {
            self.format_block_expression(block_id, span, true);
        }
    }

    /// Format a list
    fn format_list(&mut self, items: &[ListItem], _span: Span) {
        if items.is_empty() {
            self.write("[]");
            return;
        }

        // Check if all items are simple (primitives)
        let all_simple = items.iter().all(|item| match item {
            ListItem::Item(expr) => self.is_simple_expr(expr),
            ListItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        if all_simple && items.len() <= 5 {
            // Inline format
            self.write("[");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                match item {
                    ListItem::Item(expr) => self.format_expression(expr),
                    ListItem::Spread(_, expr) => {
                        self.write("...");
                        self.format_expression(expr);
                    }
                }
            }
            self.write("]");
        } else {
            // Multiline format
            self.write("[");
            self.newline();
            self.indent_level += 1;
            for item in items {
                self.write_indent();
                match item {
                    ListItem::Item(expr) => self.format_expression(expr),
                    ListItem::Spread(_, expr) => {
                        self.write("...");
                        self.format_expression(expr);
                    }
                }
                self.newline();
            }
            self.indent_level -= 1;
            self.write_indent();
            self.write("]");
        }
    }

    /// Format a record
    fn format_record(&mut self, items: &[RecordItem], _span: Span) {
        if items.is_empty() {
            self.write("{}");
            return;
        }

        // Check if all items are simple
        let all_simple = items.iter().all(|item| match item {
            RecordItem::Pair(k, v) => self.is_simple_expr(k) && self.is_simple_expr(v),
            RecordItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        if all_simple && items.len() <= 3 {
            // Inline format
            self.write("{");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                match item {
                    RecordItem::Pair(key, value) => {
                        self.format_expression(key);
                        self.write(": ");
                        self.format_expression(value);
                    }
                    RecordItem::Spread(_, expr) => {
                        self.write("...");
                        self.format_expression(expr);
                    }
                }
            }
            self.write("}");
        } else {
            // Multiline format
            self.write("{");
            self.newline();
            self.indent_level += 1;
            for item in items {
                self.write_indent();
                match item {
                    RecordItem::Pair(key, value) => {
                        self.format_expression(key);
                        self.write(": ");
                        self.format_expression(value);
                    }
                    RecordItem::Spread(_, expr) => {
                        self.write("...");
                        self.format_expression(expr);
                    }
                }
                self.newline();
            }
            self.indent_level -= 1;
            self.write_indent();
            self.write("}");
        }
    }

    /// Format a table
    fn format_table(&mut self, columns: &[Expression], rows: &[Box<[Expression]>], _span: Span) {
        self.write("[");

        // Format header row
        self.write("[");
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.format_expression(col);
        }
        self.write("]");

        // Format data rows
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

    /// Format a match block
    fn format_match_block(&mut self, matches: &[(MatchPattern, Expression)]) {
        self.write("{");
        self.newline();
        self.indent_level += 1;

        for (pattern, expr) in matches {
            self.write_indent();
            self.format_match_pattern(pattern);
            self.write(" => ");

            match &expr.expr {
                Expr::Block(block_id) | Expr::Closure(block_id) => {
                    self.format_block_expression(*block_id, expr.span, true);
                }
                _ => {
                    self.format_expression(expr);
                }
            }
            self.newline();
        }

        self.indent_level -= 1;
        self.write_indent();
        self.write("}");
    }

    /// Format a match pattern
    fn format_match_pattern(&mut self, pattern: &MatchPattern) {
        match &pattern.pattern {
            Pattern::Expression(expr) => self.format_expression(expr),
            Pattern::Value(val) => {
                // For Value patterns, use the original span content
                let content = self.get_span_content(pattern.span);
                self.write_bytes(&content);
                let _ = val; // Suppress unused warning
            }
            Pattern::Variable(_) => {
                // Use the original span content for variable patterns
                let content = self.get_span_content(pattern.span);
                self.write_bytes(&content);
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
            Pattern::Rest(_) => {
                let content = self.get_span_content(pattern.span);
                self.write_bytes(&content);
            }
            Pattern::IgnoreRest => {
                self.write("..");
            }
            Pattern::IgnoreValue => {
                self.write("_");
            }
            Pattern::Garbage => {
                // Output original content
                let content = self.get_span_content(pattern.span);
                self.write_bytes(&content);
            }
        }
    }

    /// Check if an expression is simple (primitive type)
    fn is_simple_expr(&self, expr: &Expression) -> bool {
        matches!(
            &expr.expr,
            Expr::Int(_)
                | Expr::Float(_)
                | Expr::Bool(_)
                | Expr::String(_)
                | Expr::RawString(_)
                | Expr::Nothing
                | Expr::Var(_)
                | Expr::Filepath(_, _)
                | Expr::Directory(_, _)
                | Expr::GlobPattern(_, _)
                | Expr::DateTime(_)
        )
    }

    /// Get the final output
    fn finish(self) -> Vec<u8> {
        self.output
    }
}

/// Extract comments from source code
fn extract_comments(source: &[u8]) -> Vec<(Span, Vec<u8>)> {
    let mut comments = Vec::new();
    let mut i = 0;
    let mut in_string = false;
    let mut string_char = b'"';

    while i < source.len() {
        let c = source[i];

        // Track string state to avoid matching # inside strings
        if !in_string && (c == b'"' || c == b'\'') {
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }

        if in_string {
            if c == b'\\' && i + 1 < source.len() {
                i += 2; // Skip escaped character
                continue;
            }
            if c == string_char {
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Found a comment
        if c == b'#' {
            let start = i;
            // Find end of line
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            let content = source[start..i].to_vec();
            comments.push((Span::new(start, i), content));
        }

        i += 1;
    }

    comments
}

/// Format an array of bytes
///
/// Reading the file gives you a list of bytes
pub(crate) fn format_inner(contents: &[u8], config: &Config) -> Result<Vec<u8>, FormatError> {
    let engine_state = get_engine_state();
    let mut working_set = StateWorkingSet::new(&engine_state);

    let parsed_block = parse(&mut working_set, None, contents, false);
    trace!("parsed block:\n{:?}", &parsed_block);

    // Check for parse errors (garbage)
    if has_garbage(&parsed_block, &working_set) {
        debug!("Found parsing errors, returning original content");
        return Err(FormatError::GarbageFound);
    }

    if parsed_block.pipelines.is_empty() {
        trace!("block has no pipelines!");
        debug!("File has no code to format.");
        // Still process for comments
        let comments = extract_comments(contents);
        if comments.is_empty() {
            return Ok(contents.to_vec());
        }
    }

    let mut formatter = Formatter::new(contents, &working_set, config);

    // Write leading comments
    if let Some(first_pipeline) = parsed_block.pipelines.first() {
        if let Some(first_elem) = first_pipeline.elements.first() {
            formatter.write_comments_before(first_elem.expr.span.start);
        }
    }

    formatter.format_block(&parsed_block);

    // Write trailing comments
    let end_pos = if let Some(last_pipeline) = parsed_block.pipelines.last() {
        if let Some(last_elem) = last_pipeline.elements.last() {
            last_elem.expr.span.end
        } else {
            0
        }
    } else {
        0
    };

    if end_pos > 0 {
        formatter.last_pos = end_pos;
        formatter.write_comments_before(contents.len());
    }

    Ok(formatter.finish())
}

/// Check if a block contains garbage (parse errors)
fn has_garbage(block: &Block, working_set: &StateWorkingSet) -> bool {
    for pipeline in &block.pipelines {
        for element in &pipeline.elements {
            if expr_has_garbage(&element.expr, working_set) {
                return true;
            }
        }
    }
    false
}

/// Check if an expression contains garbage
fn expr_has_garbage(expr: &Expression, working_set: &StateWorkingSet) -> bool {
    match &expr.expr {
        Expr::Garbage => true,
        Expr::BinaryOp(l, o, r) => {
            expr_has_garbage(l, working_set)
                || expr_has_garbage(o, working_set)
                || expr_has_garbage(r, working_set)
        }
        Expr::UnaryNot(e) => expr_has_garbage(e, working_set),
        Expr::Block(block_id) | Expr::Closure(block_id) | Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            has_garbage(block, working_set)
        }
        Expr::Call(call) => call.arguments.iter().any(|arg| match arg {
            Argument::Positional(e) | Argument::Unknown(e) | Argument::Spread(e) => {
                expr_has_garbage(e, working_set)
            }
            Argument::Named(n) => {
                n.2.as_ref()
                    .is_some_and(|e| expr_has_garbage(e, working_set))
            }
        }),
        Expr::List(items) => items.iter().any(|item| match item {
            ListItem::Item(e) => expr_has_garbage(e, working_set),
            ListItem::Spread(_, e) => expr_has_garbage(e, working_set),
        }),
        Expr::Record(items) => items.iter().any(|item| match item {
            RecordItem::Pair(k, v) => {
                expr_has_garbage(k, working_set) || expr_has_garbage(v, working_set)
            }
            RecordItem::Spread(_, e) => expr_has_garbage(e, working_set),
        }),
        _ => false,
    }
}

/// Make sure there is a newline at the end of a buffer
pub(crate) fn add_newline_at_end_of_file(out: Vec<u8>) -> Vec<u8> {
    match out.last() {
        Some(&b'\n') => out,
        _ => {
            let mut result = out;
            result.push(b'\n');
            result
        }
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
        // External commands are parsed when internal commands aren't available
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
}
