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
    Signature, Span, SyntaxShape,
};

/// Commands that format their block arguments in a special way
const BLOCK_COMMANDS: &[&str] = &["for", "while", "loop", "module"];
const CONDITIONAL_COMMANDS: &[&str] = &["if", "try"];
const DEF_COMMANDS: &[&str] = &["def", "def-env", "export def"];
const EXTERN_COMMANDS: &[&str] = &["extern"];
const LET_COMMANDS: &[&str] = &["let", "let-env", "mut", "const"];

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

    // ─────────────────────────────────────────────────────────────────────────────
    // Basic output methods
    // ─────────────────────────────────────────────────────────────────────────────

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

    /// Write a space if not at line start and not already following whitespace/opener
    fn space(&mut self) {
        if !self.at_line_start && !self.output.is_empty() {
            if let Some(&last) = self.output.last() {
                if !matches!(last, b' ' | b'\n' | b'\t' | b'(' | b'[') {
                    self.output.push(b' ');
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Span and source helpers
    // ─────────────────────────────────────────────────────────────────────────────

    /// Get the source content for a span (returns owned Vec to avoid borrow issues)
    fn get_span_content(&self, span: Span) -> Vec<u8> {
        self.source[span.start..span.end].to_vec()
    }

    /// Write the original source content for a span
    fn write_span(&mut self, span: Span) {
        let content = self.source[span.start..span.end].to_vec();
        self.write_bytes(&content);
    }

    /// Write the original source content for an expression's span
    fn write_expr_span(&mut self, expr: &Expression) {
        self.write_span(expr.span);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Comment handling
    // ─────────────────────────────────────────────────────────────────────────────

    /// Check if there are any comments between `last_pos` and the given position
    fn write_comments_before(&mut self, pos: usize) {
        let mut comments_to_write: Vec<_> = self
            .comments
            .iter()
            .enumerate()
            .filter(|(i, (span, _))| {
                !self.written_comments[*i] && span.start >= self.last_pos && span.end <= pos
            })
            .map(|(i, (span, content))| (i, span.start, content.clone()))
            .collect();

        comments_to_write.sort_by_key(|(_, start, _)| *start);

        for (idx, _, content) in comments_to_write {
            self.written_comments[idx] = true;
            if !self.at_line_start {
                if let Some(&last) = self.output.last() {
                    if last != b'\n' {
                        self.newline();
                    }
                }
            }
            self.write_indent();
            self.output.extend(&content);
            self.newline();
        }
    }

    /// Check for inline comment after a position (on the same line)
    fn write_inline_comment(&mut self, after_pos: usize) {
        let line_end = self.source[after_pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(self.source.len(), |p| after_pos + p);

        let found = self
            .comments
            .iter()
            .enumerate()
            .find(|(i, (span, _))| {
                !self.written_comments[*i] && span.start >= after_pos && span.start < line_end
            })
            .map(|(i, (span, content))| (i, *span, content.clone()));

        if let Some((idx, span, content)) = found {
            self.written_comments[idx] = true;
            self.write(" ");
            self.output.extend(&content);
            self.last_pos = span.end;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Block and pipeline formatting
    // ─────────────────────────────────────────────────────────────────────────────

    /// Format a block
    fn format_block(&mut self, block: &Block) {
        let num_pipelines = block.pipelines.len();
        for (i, pipeline) in block.pipelines.iter().enumerate() {
            if let Some(first_elem) = pipeline.elements.first() {
                self.write_comments_before(first_elem.expr.span.start);
            }

            self.format_pipeline(pipeline);

            if let Some(last_elem) = pipeline.elements.last() {
                let end_pos = self.get_element_end_pos(last_elem);
                self.write_inline_comment(end_pos);
                self.last_pos = end_pos;
            }

            if i < num_pipelines - 1 {
                self.newline();
            }
        }
    }

    /// Get the end position of a pipeline element, including any redirections
    fn get_element_end_pos(&self, element: &PipelineElement) -> usize {
        element
            .redirection
            .as_ref()
            .map_or(element.expr.span.end, |redir| match redir {
                PipelineRedirection::Single { target, .. } => target.span().end,
                PipelineRedirection::Separate { out, err } => out.span().end.max(err.span().end),
            })
    }

    /// Format a pipeline
    fn format_pipeline(&mut self, pipeline: &Pipeline) {
        for (i, element) in pipeline.elements.iter().enumerate() {
            if i > 0 {
                self.write(" | ");
            }
            self.format_pipeline_element(element);
        }
    }

    /// Format a pipeline element
    fn format_pipeline_element(&mut self, element: &PipelineElement) {
        self.format_expression(&element.expr);
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
                self.write_span(*span);
                self.space();
                self.format_expression(expr);
            }
            RedirectionTarget::Pipe { span } => {
                self.write_span(*span);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Expression formatting
    // ─────────────────────────────────────────────────────────────────────────────

    /// Format an expression
    fn format_expression(&mut self, expr: &Expression) {
        match &expr.expr {
            // Literals and simple values - preserve original
            Expr::Int(_)
            | Expr::Float(_)
            | Expr::Bool(_)
            | Expr::Nothing
            | Expr::DateTime(_)
            | Expr::String(_)
            | Expr::RawString(_)
            | Expr::Binary(_)
            | Expr::Filepath(_, _)
            | Expr::Directory(_, _)
            | Expr::GlobPattern(_, _)
            | Expr::Var(_)
            | Expr::VarDecl(_)
            | Expr::Operator(_)
            | Expr::StringInterpolation(_)
            | Expr::GlobInterpolation(_, _)
            | Expr::ImportPattern(_)
            | Expr::Overlay(_)
            | Expr::Garbage => {
                self.write_expr_span(expr);
            }

            Expr::Signature(sig) => self.format_signature(sig),

            Expr::Call(call) => self.format_call(call),
            Expr::ExternalCall(head, args) => self.format_external_call(head, args),
            Expr::BinaryOp(lhs, op, rhs) => self.format_binary_op(lhs, op, rhs),
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
                self.format_subexpression(*block_id);
            }

            Expr::List(items) => self.format_list(items),
            Expr::Record(items) => self.format_record(items),
            Expr::Table(table) => self.format_table(&table.columns, &table.rows),

            Expr::Range(range) => self.format_range(range),
            Expr::CellPath(cell_path) => self.format_cell_path_members(&cell_path.members),
            Expr::FullCellPath(full_path) => {
                self.format_expression(&full_path.head);
                self.format_cell_path_members(&full_path.tail);
            }

            Expr::RowCondition(block_id) => {
                let block = self.working_set.get_block(*block_id);
                self.format_block(block);
            }

            Expr::Keyword(keyword) => {
                self.write_span(keyword.span);
                self.space();
                self.format_block_or_expr(&keyword.expr);
            }

            Expr::ValueWithUnit(_) => {
                // Preserve original span since the parser normalizes units
                // (e.g., 1kb becomes 1000b internally)
                self.write_expr_span(expr);
            }

            Expr::MatchBlock(matches) => self.format_match_block(matches),

            Expr::Collect(_, inner) => self.format_expression(inner),

            Expr::AttributeBlock(attr_block) => {
                for attr in &attr_block.attributes {
                    self.write_span(attr.expr.span);
                    self.newline();
                }
                self.format_expression(&attr_block.item);
            }
        }
    }

    /// Format a call expression
    fn format_call(&mut self, call: &nu_protocol::ast::Call) {
        let decl = self.working_set.get_decl(call.decl_id);
        let decl_name = decl.name();

        // Determine command type
        let cmd_type = Self::classify_command(decl_name);

        // Write command name
        if call.head.end != 0 {
            self.write_span(call.head);
        }

        // Format arguments based on command type
        for arg in &call.arguments {
            self.format_call_argument(arg, &cmd_type);
        }
    }

    /// Classify a command by its formatting requirements
    fn classify_command(name: &str) -> CommandType {
        if DEF_COMMANDS.contains(&name) {
            CommandType::Def
        } else if EXTERN_COMMANDS.contains(&name) {
            CommandType::Extern
        } else if CONDITIONAL_COMMANDS.contains(&name) {
            CommandType::Conditional
        } else if LET_COMMANDS.contains(&name) {
            CommandType::Let
        } else if BLOCK_COMMANDS.contains(&name) {
            CommandType::Block
        } else {
            CommandType::Regular
        }
    }

    /// Format a call argument based on command type
    fn format_call_argument(&mut self, arg: &Argument, cmd_type: &CommandType) {
        match arg {
            Argument::Positional(positional) | Argument::Unknown(positional) => {
                self.format_positional_argument(positional, cmd_type);
            }
            Argument::Named(named) => {
                self.space();
                if named.0.span.end != 0 {
                    self.write_span(named.0.span);
                }
                if let Some(short) = &named.1 {
                    self.write_span(short.span);
                }
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

    /// Format a positional argument based on command type
    fn format_positional_argument(&mut self, positional: &Expression, cmd_type: &CommandType) {
        self.space();
        match cmd_type {
            CommandType::Def => self.format_def_argument(positional),
            CommandType::Extern => self.format_extern_argument(positional),
            CommandType::Conditional | CommandType::Block => {
                self.format_block_or_expr(positional);
            }
            CommandType::Let => self.format_let_argument(positional),
            CommandType::Regular => self.format_expression(positional),
        }
    }

    /// Format an argument for def commands
    fn format_def_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::String(_) => self.format_expression(positional),
            Expr::Signature(sig) => self.format_signature(sig),
            Expr::Closure(block_id) | Expr::Block(block_id) => {
                self.format_block_expression(*block_id, positional.span, true);
            }
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for extern commands (preserve original signature)
    fn format_extern_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            // For extern, preserve the signature span to maintain parameter order
            Expr::Signature(_) => self.write_expr_span(positional),
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for let/mut/const commands
    fn format_let_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::VarDecl(_) => self.format_expression(positional),
            Expr::Block(block_id) | Expr::Subexpression(block_id) => {
                self.write("= ");
                let block = self.working_set.get_block(*block_id);
                self.format_block(block);
            }
            _ => {
                self.write("= ");
                self.format_expression(positional);
            }
        }
    }

    /// Format an expression that could be a block or a regular expression
    fn format_block_or_expr(&mut self, expr: &Expression) {
        match &expr.expr {
            Expr::Block(block_id) | Expr::Closure(block_id) => {
                self.format_block_expression(*block_id, expr.span, true);
            }
            _ => self.format_expression(expr),
        }
    }

    /// Format an external call
    fn format_external_call(&mut self, head: &Expression, args: &[ExternalArgument]) {
        // Check if the original source had an explicit ^ prefix
        // by looking at the byte before the head span
        if head.span.start > 0 && self.source.get(head.span.start - 1) == Some(&b'^') {
            self.write("^");
        }
        self.format_expression(head);
        for arg in args {
            self.space();
            match arg {
                ExternalArgument::Regular(arg_expr) => self.format_expression(arg_expr),
                ExternalArgument::Spread(spread_expr) => {
                    self.write("...");
                    self.format_expression(spread_expr);
                }
            }
        }
    }

    /// Format a binary operation
    fn format_binary_op(&mut self, lhs: &Expression, op: &Expression, rhs: &Expression) {
        self.format_expression(lhs);
        // Always add space around binary operators for valid nushell syntax
        self.write(" ");
        self.format_expression(op);
        self.write(" ");

        // For assignment operators, unwrap Subexpression on RHS to avoid double parens
        if let Expr::Operator(nu_protocol::ast::Operator::Assignment(_)) = &op.expr {
            if let Expr::Subexpression(block_id) = &rhs.expr {
                let block = self.working_set.get_block(*block_id);
                self.format_block(block);
                return;
            }
        }
        self.format_expression(rhs);
    }

    /// Format a range expression
    fn format_range(&mut self, range: &nu_protocol::ast::Range) {
        if let Some(from) = &range.from {
            self.format_expression(from);
        }
        self.write("..");
        if let Some(next) = &range.next {
            self.format_expression(next);
            // For step ranges (start..step..end), write the operator again before end
            self.write("..");
        }
        if let Some(to) = &range.to {
            self.format_expression(to);
        }
    }

    /// Format a signature (for def commands)
    fn format_signature(&mut self, sig: &Signature) {
        self.write("[");

        let param_count = sig.required_positional.len()
            + sig.optional_positional.len()
            + sig.named.iter().filter(|f| f.long != "help").count()
            + if sig.rest_positional.is_some() { 1 } else { 0 };
        let has_multiline = param_count > 3;

        if has_multiline {
            self.newline();
            self.indent_level += 1;
        }

        let mut first = true;

        // Helper to write separator
        let write_sep = |formatter: &mut Formatter, first: &mut bool, has_multiline: bool| {
            if !*first {
                if has_multiline {
                    formatter.newline();
                    formatter.write_indent();
                } else {
                    formatter.write(", ");
                }
            }
            *first = false;
        };

        // Required positional
        for param in &sig.required_positional {
            write_sep(self, &mut first, has_multiline);
            self.write(&param.name);
            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write(&format!("{}", param.shape));
            }
        }

        // Optional positional
        for param in &sig.optional_positional {
            write_sep(self, &mut first, has_multiline);
            self.write(&param.name);
            // If there's a default value, don't use ? syntax, use = syntax
            if param.default_value.is_none() {
                self.write("?");
            }
            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write(&format!("{}", param.shape));
            }
            if let Some(default) = &param.default_value {
                self.write(" = ");
                self.write(&default.to_expanded_string(" ", &nu_protocol::Config::default()));
            }
        }

        // Named flags (before rest positional to match common convention)
        for flag in &sig.named {
            // Skip help flag as it's auto-added
            if flag.long == "help" {
                continue;
            }
            write_sep(self, &mut first, has_multiline);

            // Handle short-only flags (empty long name)
            if flag.long.is_empty() {
                if let Some(short) = flag.short {
                    self.write("-");
                    self.write(&short.to_string());
                }
            } else {
                self.write("--");
                self.write(&flag.long);
                if let Some(short) = flag.short {
                    self.write("(-");
                    self.write(&short.to_string());
                    self.write(")");
                }
            }
            if let Some(shape) = &flag.arg {
                self.write(": ");
                self.write(&format!("{}", shape));
            }
            if let Some(default) = &flag.default_value {
                self.write(" = ");
                self.write(&default.to_expanded_string(" ", &nu_protocol::Config::default()));
            }
        }

        // Rest positional (comes last)
        if let Some(rest) = &sig.rest_positional {
            write_sep(self, &mut first, has_multiline);
            self.write("...");
            self.write(&rest.name);
            if rest.shape != SyntaxShape::Any {
                self.write(": ");
                self.write(&format!("{}", rest.shape));
            }
        }

        if has_multiline {
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }
        self.write("]");
    }

    /// Format cell path members (shared between `CellPath` and `FullCellPath`)
    fn format_cell_path_members(&mut self, members: &[PathMember]) {
        for member in members {
            self.write(".");
            match member {
                PathMember::String { val, optional, .. } => {
                    if *optional {
                        self.write("?");
                    }
                    self.write(val);
                }
                PathMember::Int { val, optional, .. } => {
                    if *optional {
                        self.write("?");
                    }
                    self.write(&val.to_string());
                }
            }
        }
    }

    /// Format a subexpression
    fn format_subexpression(&mut self, block_id: nu_protocol::BlockId) {
        let block = self.working_set.get_block(block_id);
        // Special case: subexpressions containing only a string interpolation don't need parentheses
        if block.pipelines.len() == 1 && block.pipelines[0].elements.len() == 1 {
            if let Expr::StringInterpolation(_) = &block.pipelines[0].elements[0].expr.expr {
                self.format_block(block);
                return;
            }
        }

        self.write("(");
        let is_simple = block.pipelines.len() == 1 && block.pipelines[0].elements.len() <= 3;

        if is_simple {
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Block expression formatting
    // ─────────────────────────────────────────────────────────────────────────────

    /// Format a block expression with optional braces
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

        let is_simple = block.pipelines.len() == 1
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block);

        if is_simple && with_braces {
            self.write(" ");
            self.format_block(block);
            self.write(" ");
        } else if block.pipelines.is_empty() {
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
        block
            .pipelines
            .iter()
            .flat_map(|p| &p.elements)
            .any(|e| self.expr_is_complex(&e.expr))
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
        let has_params = content.starts_with(b"{|") || content.starts_with(b"{ |");

        if !has_params {
            self.format_block_expression(block_id, span, true);
            return;
        }

        // Find the end of the parameter section (second |)
        let param_end = content.iter().position(|&b| b == b'|').and_then(|first| {
            content[first + 1..]
                .iter()
                .position(|&b| b == b'|')
                .map(|p| first + 1 + p + 1)
        });

        let Some(end) = param_end else {
            self.write_bytes(&content);
            return;
        };

        self.write("{|");
        // Extract and trim parameter content
        let params = &content[2..end - 1];
        let trimmed: Vec<u8> = params
            .iter()
            .copied()
            .skip_while(|b| b.is_ascii_whitespace())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .skip_while(|b| b.is_ascii_whitespace())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        self.write_bytes(&trimmed);
        self.write("| ");

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
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Collection formatting (lists, records, tables)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Format a list
    fn format_list(&mut self, items: &[ListItem]) {
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
                self.format_list_item(item);
            }
            self.write("]");
        } else {
            // Multiline format
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

    /// Format a single list item
    fn format_list_item(&mut self, item: &ListItem) {
        match item {
            ListItem::Item(expr) => self.format_expression(expr),
            ListItem::Spread(_, expr) => {
                self.write("...");
                self.format_expression(expr);
            }
        }
    }

    /// Format a record
    fn format_record(&mut self, items: &[RecordItem]) {
        if items.is_empty() {
            self.write("{}");
            return;
        }

        // Check if all items are simple
        let all_simple = items.iter().all(|item| match item {
            RecordItem::Pair(k, v) => self.is_simple_expr(k) && self.is_simple_expr(v),
            RecordItem::Spread(_, expr) => self.is_simple_expr(expr),
        });

        // Check if any value contains nested structures (records, lists, closures) or variables
        let has_nested_complex = items.iter().any(|item| match item {
            RecordItem::Pair(_, v) => matches!(
                &v.expr,
                Expr::Record(_)
                    | Expr::List(_)
                    | Expr::Closure(_)
                    | Expr::Block(_)
                    | Expr::Var(_)
                    | Expr::FullCellPath(_)
            ),
            RecordItem::Spread(_, _) => false,
        });

        // When nested with complex values or variables, records with 2+ items should be multiline
        let nested_multiline = self.indent_level > 0 && items.len() >= 2 && has_nested_complex;

        if all_simple && items.len() <= 3 && !nested_multiline {
            // Inline format
            self.write("{");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.format_record_item(item);
            }
            self.write("}");
        } else {
            // Multiline format
            self.write("{");
            self.newline();
            self.indent_level += 1;
            for item in items {
                self.write_indent();
                self.format_record_item(item);
                self.newline();
            }
            self.indent_level -= 1;
            self.write_indent();
            self.write("}");
        }
    }

    /// Format a single record item
    fn format_record_item(&mut self, item: &RecordItem) {
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

    /// Format a table
    fn format_table(&mut self, columns: &[Expression], rows: &[Box<[Expression]>]) {
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Match block formatting
    // ─────────────────────────────────────────────────────────────────────────────

    /// Format a match block
    fn format_match_block(&mut self, matches: &[(MatchPattern, Expression)]) {
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

    /// Format a match pattern
    fn format_match_pattern(&mut self, pattern: &MatchPattern) {
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────────

    /// Check if an expression is simple (primitive type)
    fn is_simple_expr(&self, expr: &Expression) -> bool {
        match &expr.expr {
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
            | Expr::DateTime(_) => true,
            // FullCellPath with empty tail is simple (e.g., $var or undefined $var parsed as Garbage)
            Expr::FullCellPath(full_path) => {
                full_path.tail.is_empty()
                    && matches!(
                        &full_path.head.expr,
                        Expr::Var(_) | Expr::Garbage | Expr::Int(_) | Expr::String(_)
                    )
            }
            _ => false,
        }
    }

    /// Get the final output
    fn finish(self) -> Vec<u8> {
        self.output
    }
}

/// Command types for formatting purposes
enum CommandType {
    Def,
    Extern,
    Conditional,
    Let,
    Block,
    Regular,
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
            while i < source.len() && source[i] != b'\n' {
                i += 1;
            }
            comments.push((Span::new(start, i), source[start..i].to_vec()));
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

    let mut formatter = Formatter::new(contents, &working_set, config);

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

/// Make sure there is a newline at the end of a buffer
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
}
