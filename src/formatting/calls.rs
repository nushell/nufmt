//! Call and argument formatting.
//!
//! Handles `def`, `let`/`mut`/`const`, `extern`, conditional, and regular
//! command calls, including signature rendering and custom completions.

use super::{CommandType, Formatter};
use nu_protocol::{
    ast::{Argument, Expr, Expression, ExternalArgument},
    Completion, Signature, SyntaxShape,
};
use nu_utils::NuCow;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Commands whose block arguments are formatted specially.
pub(super) const BLOCK_COMMANDS: &[&str] = &["for", "while", "loop", "module"];
pub(super) const CONDITIONAL_COMMANDS: &[&str] = &["if", "try"];
pub(super) const DEF_COMMANDS: &[&str] = &["def", "def-env", "export def"];
pub(super) const EXTERN_COMMANDS: &[&str] = &["extern", "export extern"];
pub(super) const ALIAS_COMMANDS: &[&str] = &["alias", "export alias"];
pub(super) const LET_COMMANDS: &[&str] = &["let", "let-env", "mut", "const", "export const"];

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Call formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a call expression.
    pub(super) fn format_call(&mut self, call: &nu_protocol::ast::Call) {
        let decl = self.working_set.get_decl(call.decl_id);
        let decl_name = decl.name();
        let cmd_type = Self::classify_command(decl_name);
        let head_text = self.call_head_text(call);

        if self.should_wrap_call_multiline(call, &cmd_type) {
            self.format_wrapped_call(call);
            return;
        }

        // Write command name
        if call.head.end != 0 {
            self.write_span(call.head);
        }

        if matches!(cmd_type, CommandType::Let) {
            self.format_let_call(call);
            return;
        }

        if matches!(cmd_type, CommandType::Alias) {
            if self.call_head_matches_alias_decl_command(call) {
                self.format_alias_call(call);
            } else {
                self.format_alias_invocation_call(call);
            }
            return;
        }

        if matches!(cmd_type, CommandType::Regular)
            && head_text.as_deref().is_some_and(|head| head != decl_name)
        {
            self.format_alias_invocation_call(call);
            return;
        }

        if decl_name == "for" {
            self.format_for_call(call);
            return;
        }

        let preserve_not_subexpr_parens = self.conditional_context_depth > 0 && decl_name == "not";

        if preserve_not_subexpr_parens {
            self.preserve_subexpr_parens_depth += 1;
        }

        for arg in &call.arguments {
            if matches!(cmd_type, CommandType::Regular)
                && !self.argument_belongs_to_call_source(call, arg)
            {
                continue;
            }
            self.format_call_argument(arg, &cmd_type);
        }

        if preserve_not_subexpr_parens {
            self.preserve_subexpr_parens_depth -= 1;
        }
    }

    /// Decide if a call should be emitted as a parenthesized multiline call.
    fn should_wrap_call_multiline(
        &self,
        call: &nu_protocol::ast::Call,
        cmd_type: &CommandType,
    ) -> bool {
        if !matches!(cmd_type, CommandType::Regular) {
            return false;
        }

        let source_args: Vec<&Argument> = call
            .arguments
            .iter()
            .filter(|arg| self.argument_belongs_to_call_source(call, arg))
            .collect();

        if source_args.len() < 3 {
            return false;
        }

        if !source_args.iter().all(|arg| {
            matches!(
                *arg,
                Argument::Positional(_) | Argument::Unknown(_) | Argument::Spread(_)
            )
        }) {
            return false;
        }

        let end = source_args
            .iter()
            .map(|arg| match *arg {
                Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                    expr.span.end
                }
                Argument::Named(named) => named
                    .2
                    .as_ref()
                    .map_or(named.0.span.end, |value| value.span.end),
            })
            .max()
            .unwrap_or(call.head.end);

        if call.head.start >= end || end > self.source.len() {
            return false;
        }

        let source_span = &self.source[call.head.start..end];
        if source_span.contains(&b'\n') {
            return false;
        }

        source_span.len() > self.config.line_length
    }

    /// Format a long regular call as:
    ///
    /// `(cmd\n  arg1\n  arg2\n)`
    fn format_wrapped_call(&mut self, call: &nu_protocol::ast::Call) {
        let source_args: Vec<&Argument> = call
            .arguments
            .iter()
            .filter(|arg| self.argument_belongs_to_call_source(call, arg))
            .collect();

        self.write("(");
        if call.head.end != 0 {
            self.write_span(call.head);
        }
        self.newline();
        self.indent_level += 1;

        for arg in source_args {
            self.write_indent();
            match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) => {
                    self.format_expression(expr);
                }
                Argument::Spread(expr) => {
                    self.write("...");
                    self.format_expression(expr);
                }
                Argument::Named(_) => {
                    // Guarded out by should_wrap_call_multiline.
                    self.format_call_argument(arg, &CommandType::Regular);
                }
            }
            self.newline();
        }

        self.indent_level -= 1;
        self.write_indent();
        self.write(")");
    }

    fn call_head_text(&self, call: &nu_protocol::ast::Call) -> Option<String> {
        if call.head.end <= call.head.start || call.head.end > self.source.len() {
            return None;
        }

        Some(
            String::from_utf8_lossy(&self.source[call.head.start..call.head.end])
                .trim()
                .to_string(),
        )
    }

    fn call_head_matches_alias_decl_command(&self, call: &nu_protocol::ast::Call) -> bool {
        self.call_head_text(call)
            .as_deref()
            .is_some_and(|head| ALIAS_COMMANDS.contains(&head))
    }

    fn format_alias_invocation_call(&mut self, call: &nu_protocol::ast::Call) {
        let call_end = call
            .arguments
            .iter()
            .map(|arg| match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                    expr.span.end
                }
                Argument::Named(named) => named
                    .2
                    .as_ref()
                    .map_or(named.0.span.end, |value| value.span.end),
            })
            .max()
            .unwrap_or(call.head.end)
            .min(self.source.len());

        if call.head.end < call_end {
            self.write_bytes(&self.source[call.head.end..call_end]);
        }
    }

    fn argument_belongs_to_call_source(
        &self,
        call: &nu_protocol::ast::Call,
        arg: &Argument,
    ) -> bool {
        let span_start = match arg {
            Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                expr.span.start
            }
            Argument::Named(named) => named.0.span.start,
        };

        span_start >= call.head.start
    }

    /// Format `let`/`mut`/`const` calls while preserving explicit type annotations.
    pub(super) fn format_let_call(&mut self, call: &nu_protocol::ast::Call) {
        let positional: Vec<&Expression> = call
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) => Some(expr),
                _ => None,
            })
            .collect();

        if positional.is_empty() {
            for arg in &call.arguments {
                self.format_call_argument(arg, &CommandType::Let);
            }
            return;
        }

        self.space();
        self.format_expression(positional[0]);

        if let Some(rhs) = positional.get(1) {
            let lhs = positional[0];
            let between = if lhs.span.end <= rhs.span.start {
                &self.source[lhs.span.end..rhs.span.start]
            } else {
                &[]
            };

            if let Some(eq_pos) = between.iter().position(|b| *b == b'=') {
                let annotation = between[..eq_pos].trim_ascii();
                if !annotation.is_empty() {
                    if !annotation.starts_with(b":") {
                        self.space();
                    }
                    self.write_bytes(annotation);
                }
            }

            self.write(" = ");

            match &rhs.expr {
                Expr::Subexpression(block_id) => {
                    self.format_assignment_subexpression(*block_id, rhs.span);
                }
                Expr::Block(block_id) => {
                    let block = self.working_set.get_block(*block_id);
                    self.format_block(block);
                }
                _ => {
                    if !self.try_write_redundant_parenthesized_pipeline_rhs(rhs) {
                        self.format_expression(rhs);
                    }
                }
            }

            for extra in positional.iter().skip(2) {
                self.space();
                self.format_expression(extra);
            }
        }

        // Emit any non-positional arguments (e.g. named flags)
        for arg in &call.arguments {
            if !matches!(arg, Argument::Positional(_) | Argument::Unknown(_)) {
                self.format_call_argument(arg, &CommandType::Let);
            }
        }
    }

    /// Format `for` loop calls, preserving explicit type annotations on the
    /// loop variable (e.g. `for h: int in [1 2 3] { ... }`).
    pub(super) fn format_for_call(&mut self, call: &nu_protocol::ast::Call) {
        // Find the VarDecl positional and the Keyword("in") argument.
        let var_decl = call.arguments.iter().find_map(|arg| match arg {
            Argument::Positional(expr) | Argument::Unknown(expr)
                if matches!(expr.expr, Expr::VarDecl(_)) =>
            {
                Some(expr)
            }
            _ => None,
        });

        let keyword_in = call.arguments.iter().find_map(|arg| match arg {
            Argument::Positional(expr) | Argument::Unknown(expr)
                if matches!(expr.expr, Expr::Keyword(_)) =>
            {
                Some(expr)
            }
            _ => None,
        });

        if let (Some(var_decl), Some(kw_in)) = (var_decl, keyword_in) {
            self.space();
            self.format_expression(var_decl);

            // Preserve a type annotation (e.g. `: int`) between the loop
            // variable and the `in` keyword.
            if var_decl.span.end < kw_in.span.start {
                let between = &self.source[var_decl.span.end..kw_in.span.start];
                let annotation = between.trim_ascii();
                if annotation.starts_with(b":") {
                    self.write_bytes(annotation);
                }
            }

            self.space();
            self.format_expression(kw_in);
        } else {
            // Fallback — format all arguments normally
            for arg in &call.arguments {
                self.format_call_argument(arg, &CommandType::Block);
            }
            return;
        }

        // Format remaining arguments (the body block)
        for arg in &call.arguments {
            match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) => {
                    if matches!(expr.expr, Expr::VarDecl(_) | Expr::Keyword(_)) {
                        continue;
                    }
                    self.space();
                    self.format_block_or_expr(expr);
                }
                _ => {
                    self.format_call_argument(arg, &CommandType::Block);
                }
            }
        }
    }

    /// Format alias definitions while preserving the literal right-hand side.
    ///
    /// The parser resolves alias references semantically, which can expand the
    /// RHS if it is re-rendered from the AST. Preserve the original source text
    /// after `=` to keep alias definitions idempotent.
    pub(super) fn format_alias_call(&mut self, call: &nu_protocol::ast::Call) {
        let positional: Vec<&Expression> = call
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) => Some(expr),
                _ => None,
            })
            .collect();

        let Some(name) = positional.first() else {
            for arg in &call.arguments {
                self.format_call_argument(arg, &CommandType::Alias);
            }
            return;
        };

        self.space();
        self.format_expression(name);

        let rhs_end = call
            .arguments
            .iter()
            .filter_map(|arg| match arg {
                Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                    Some(expr.span.end)
                }
                Argument::Named(named) => named
                    .2
                    .as_ref()
                    .map_or(Some(named.0.span.end), |value| Some(value.span.end)),
            })
            .max();

        let Some(rhs_end) = rhs_end else {
            return;
        };

        if name.span.end >= rhs_end || rhs_end > self.source.len() {
            self.format_regular_arguments(&call.arguments[1..]);
            return;
        }

        let between = &self.source[name.span.end..rhs_end];
        let Some(eq_offset) = between.iter().position(|byte| *byte == b'=') else {
            self.format_regular_arguments(&call.arguments[1..]);
            return;
        };

        let rhs_start = between[eq_offset + 1..]
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .map(|offset| name.span.end + eq_offset + 1 + offset);

        let Some(rhs_start) = rhs_start else {
            self.write(" =");
            return;
        };

        // Keep the exact user-authored alias RHS to avoid semantic expansion
        // when aliases reference other aliases.
        self.write(" = ");
        self.write_bytes(&self.source[rhs_start..rhs_end]);
    }

    fn format_regular_arguments(&mut self, args: &[Argument]) {
        for arg in args {
            self.format_call_argument(arg, &CommandType::Regular);
        }
    }

    /// Classify a command name into a [`CommandType`] for formatting purposes.
    pub(super) fn classify_command(name: &str) -> CommandType {
        if DEF_COMMANDS.contains(&name) {
            CommandType::Def
        } else if EXTERN_COMMANDS.contains(&name) {
            CommandType::Extern
        } else if ALIAS_COMMANDS.contains(&name) {
            CommandType::Alias
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

    // ─────────────────────────────────────────────────────────────────────────
    // Argument formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a single call argument, dispatching by [`CommandType`].
    pub(super) fn format_call_argument(&mut self, arg: &Argument, cmd_type: &CommandType) {
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
                    let separator_start = named
                        .1
                        .as_ref()
                        .map_or(named.0.span.end, |short| short.span.end);
                    let has_equals = separator_start <= value.span.start
                        && self.source[separator_start..value.span.start].contains(&b'=');

                    if has_equals {
                        self.write("=");
                    } else {
                        self.space();
                    }
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

    /// Format a positional argument, using the command type to pick the
    /// right strategy.
    fn format_positional_argument(&mut self, positional: &Expression, cmd_type: &CommandType) {
        self.space();
        match cmd_type {
            CommandType::Def => self.format_def_argument(positional),
            CommandType::Extern => self.format_extern_argument(positional),
            CommandType::Alias => self.format_expression(positional),
            CommandType::Conditional => {
                self.conditional_context_depth += 1;
                self.format_block_or_expr(positional);
                self.conditional_context_depth -= 1;
            }
            CommandType::Block => {
                self.format_block_or_expr(positional);
            }
            CommandType::Let => self.format_let_argument(positional),
            CommandType::Regular => {
                if self.try_format_empty_braced_regular_argument(positional) {
                    return;
                }

                if !self.try_format_closure_like_span(positional.span) {
                    self.format_expression(positional);
                }
            }
        }
    }

    /// Format an argument for `def` commands (name, signature, body).
    fn format_def_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::String(_) => self.format_expression(positional),
            Expr::Signature(sig) => {
                if self.has_comments_in_span(positional.span.start, positional.span.end) {
                    self.write_expr_span(positional);
                    self.mark_comments_written_in_span(positional.span.start, positional.span.end);
                } else {
                    self.format_signature(sig);
                }
            }
            Expr::Closure(block_id) | Expr::Block(block_id) => {
                self.format_block_expression(*block_id, positional.span, true);
            }
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for `extern` commands (preserve original signature).
    fn format_extern_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::Signature(_) => self.write_expr_span(positional),
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for `let`/`mut`/`const` commands.
    fn format_let_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::VarDecl(_) => self.format_expression(positional),
            Expr::Subexpression(block_id) => {
                self.write("= ");
                self.format_assignment_subexpression(*block_id, positional.span);
            }
            Expr::Block(block_id) => {
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

    /// Format let-assignment subexpressions, flattening redundant outer
    /// parentheses around pipeline-leading subexpressions such as
    /// `((pwd) | path join ...)`.
    fn format_assignment_subexpression(
        &mut self,
        block_id: nu_protocol::BlockId,
        span: nu_protocol::Span,
    ) {
        let block = self.working_set.get_block(block_id);
        if block.pipelines.len() == 1 {
            let pipeline = &block.pipelines[0];
            if pipeline.elements.len() > 1
                && matches!(pipeline.elements[0].expr.expr, Expr::Subexpression(_))
            {
                if let Expr::Subexpression(inner_id) = &pipeline.elements[0].expr.expr {
                    let inner = self.working_set.get_block(*inner_id);
                    if inner.pipelines.len() == 1 && inner.pipelines[0].elements.len() == 1 {
                        self.format_pipeline_element(&inner.pipelines[0].elements[0]);
                        for element in pipeline.elements.iter().skip(1) {
                            self.write(" | ");
                            self.format_pipeline_element(element);
                        }
                        return;
                    }
                }
            }

            if !self.pipeline_requires_multiline(pipeline) {
                self.format_block(block);
                return;
            }
        }

        self.format_subexpression(block_id, span);
    }

    /// Format an external call (e.g. `^git status`).
    pub(super) fn format_external_call(&mut self, head: &Expression, args: &[ExternalArgument]) {
        // Preserve explicit `^` prefix
        if head.span.start > 0 && self.source.get(head.span.start - 1) == Some(&b'^') {
            self.write("^");
        }
        self.format_expression(head);

        let tail_end = args
            .iter()
            .map(|arg| match arg {
                ExternalArgument::Regular(arg_expr) | ExternalArgument::Spread(arg_expr) => {
                    arg_expr.span.end
                }
            })
            .max()
            .unwrap_or(head.span.end)
            .min(self.source.len());

        if head.span.end < tail_end
            && self.external_call_tail_matches_arg_suffix(head.span.end, tail_end, args)
        {
            // Keep the authored tail only when the parser has prepended alias-expanded
            // arguments ahead of the user-written suffix.
            self.write_bytes(&self.source[head.span.end..tail_end]);
            return;
        }

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

    fn external_call_tail_matches_arg_suffix(
        &self,
        tail_start: usize,
        tail_end: usize,
        args: &[ExternalArgument],
    ) -> bool {
        let source_tokens = self.tokenize_source_words(&self.source[tail_start..tail_end]);
        let arg_tokens: Vec<Vec<u8>> = args
            .iter()
            .map(|arg| match arg {
                ExternalArgument::Regular(expr) => {
                    self.source[expr.span.start..expr.span.end].to_vec()
                }
                ExternalArgument::Spread(expr) => {
                    let mut token = b"...".to_vec();
                    token.extend_from_slice(&self.source[expr.span.start..expr.span.end]);
                    token
                }
            })
            .collect();

        if source_tokens.is_empty() || source_tokens.len() >= arg_tokens.len() {
            return false;
        }

        let suffix_start = arg_tokens.len() - source_tokens.len();
        arg_tokens[suffix_start..] == source_tokens
    }

    fn tokenize_source_words(&self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut tokens = Vec::new();
        let mut current = Vec::new();
        let mut in_string: Option<u8> = None;
        let mut escaped = false;

        for &byte in bytes {
            if let Some(quote) = in_string {
                current.push(byte);
                if escaped {
                    escaped = false;
                    continue;
                }
                if byte == b'\\' {
                    escaped = true;
                    continue;
                }
                if byte == quote {
                    in_string = None;
                }
                continue;
            }

            if byte.is_ascii_whitespace() {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                continue;
            }

            if byte == b'\'' || byte == b'"' {
                in_string = Some(byte);
            }
            current.push(byte);
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    fn try_write_redundant_parenthesized_pipeline_rhs(&mut self, rhs: &Expression) -> bool {
        let raw = self.get_span_content(rhs.span);
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 3 || trimmed.contains(&b'\n') {
            return false;
        }

        if !(trimmed.starts_with(b"(") && trimmed.ends_with(b")") && trimmed.contains(&b'|')) {
            return false;
        }

        let inner = &trimmed[1..trimmed.len() - 1];
        let inner = inner.trim_ascii();

        // Keep explicit wrappers for external-command assignments.
        if inner.starts_with(b"^") {
            return false;
        }

        if inner.is_empty() {
            return false;
        }

        self.write_bytes(inner);
        true
    }

    fn try_format_empty_braced_regular_argument(&mut self, positional: &Expression) -> bool {
        if !matches!(positional.expr, Expr::Block(_) | Expr::Closure(_)) {
            return false;
        }

        if positional.span.end <= positional.span.start + 1
            || positional.span.end > self.source.len()
        {
            return false;
        }

        let raw = &self.source[positional.span.start..positional.span.end];
        if !raw.starts_with(b"{") || !raw.ends_with(b"}") {
            return false;
        }

        if !raw[1..raw.len() - 1]
            .iter()
            .all(|b| b.is_ascii_whitespace())
        {
            return false;
        }

        if self.has_comments_in_span(positional.span.start, positional.span.end) {
            return false;
        }

        self.write("{}");
        true
    }

    fn try_format_closure_like_span(&mut self, span: nu_protocol::Span) -> bool {
        if span.end <= span.start + 2 || span.end > self.source.len() {
            return false;
        }

        let raw = &self.source[span.start..span.end];
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 4 || trimmed.contains(&b'\n') {
            return false;
        }

        if trimmed
            .get(1)
            .is_none_or(|byte| !byte.is_ascii_whitespace())
        {
            return false;
        }

        if !(trimmed.starts_with(b"{") && trimmed.ends_with(b"}")) {
            return false;
        }

        let inner = trimmed[1..trimmed.len() - 1].trim_ascii();
        if inner.first() != Some(&b'|') {
            return false;
        }

        let Some(second_pipe) = inner[1..]
            .iter()
            .position(|byte| *byte == b'|')
            .map(|pos| pos + 1)
        else {
            return false;
        };

        let params = &inner[1..second_pipe];
        let body = inner[second_pipe + 1..].trim_ascii();

        self.write("{|");
        let mut params_iter = params.split(|&b| b == b',').peekable();
        while let Some(param) = params_iter.next() {
            let mut sub_parts = param.splitn(2, |&b| b == b':');

            if let (Some(param_name), Some(type_hint)) = (sub_parts.next(), sub_parts.next()) {
                self.write_bytes(param_name.trim_ascii());
                self.write_bytes(b": ");
                self.write_bytes(type_hint.trim_ascii());
            } else {
                self.write_bytes(param.trim_ascii());
            }

            if params_iter.peek().is_some() {
                self.write_bytes(b", ");
            }
        }
        self.write("|");

        if !body.is_empty() {
            self.space();
            self.write_bytes(body);
            self.write(" ");
        }

        self.write("}");
        true
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Signature formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a parameter signature (`[x: int, --flag(-f)]`).
    pub(super) fn format_signature(&mut self, sig: &Signature) {
        self.write("[");

        let param_count = sig.required_positional.len()
            + sig.optional_positional.len()
            + sig.named.iter().filter(|f| f.long != "help").count()
            + usize::from(sig.rest_positional.is_some());
        let has_multiline = if self.should_keep_simple_signature_inline(sig) {
            false
        } else {
            param_count > 3
        };

        if has_multiline {
            self.newline();
            self.indent_level += 1;
        }

        let mut first = true;

        // Helper closure for separators
        let write_sep = |f: &mut Formatter, first: &mut bool, multiline: bool| {
            if !*first {
                if multiline {
                    f.newline();
                    f.write_indent();
                } else {
                    f.write(", ");
                }
            }
            *first = false;
        };

        // Required positional parameters
        for param in &sig.required_positional {
            write_sep(self, &mut first, has_multiline);
            self.write(&param.name);
            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&param.shape);
                self.write_custom_completion(&param.completion);
            }
        }

        // Optional positional parameters
        for param in &sig.optional_positional {
            write_sep(self, &mut first, has_multiline);
            self.write(&param.name);
            if param.default_value.is_none() {
                self.write("?");
            }
            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&param.shape);
                self.write_custom_completion(&param.completion);
            }
            if let Some(default) = &param.default_value {
                self.write(" = ");
                self.write(&default.to_expanded_string(" ", &nu_protocol::Config::default()));
            }
        }

        // Named flags (skip auto-added --help)
        for flag in &sig.named {
            if flag.long == "help" {
                continue;
            }
            write_sep(self, &mut first, has_multiline);

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
                self.write_shape(shape);
                self.write_custom_completion(&flag.completion);
            }
            if let Some(default) = &flag.default_value {
                self.write(" = ");
                self.write(&default.to_expanded_string(" ", &nu_protocol::Config::default()));
            }
        }

        // Rest positional (last)
        if let Some(rest) = &sig.rest_positional {
            write_sep(self, &mut first, has_multiline);
            self.write("...");
            self.write(&rest.name);
            if rest.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&rest.shape);
                self.write_custom_completion(&rest.completion);
            }
        }

        if has_multiline {
            self.newline();
            self.indent_level -= 1;
            self.write_indent();
        }
        self.write("]");

        // Input/output type annotations
        if !sig.input_output_types.is_empty() {
            self.write(": ");
            for (i, (input, output)) in sig.input_output_types.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&input.to_string());
                self.write(" -> ");
                self.write(&output.to_string());
            }
        }
    }

    /// Keep simple required-positional signatures inline when they fit the
    /// configured line length.
    fn should_keep_simple_signature_inline(&self, sig: &Signature) -> bool {
        if sig.required_positional.is_empty()
            || !sig.optional_positional.is_empty()
            || sig.rest_positional.is_some()
            || !sig.input_output_types.is_empty()
            || sig.named.iter().any(|flag| flag.long != "help")
        {
            return false;
        }

        if sig
            .required_positional
            .iter()
            .any(|param| param.shape != SyntaxShape::Any || param.completion.is_some())
        {
            return false;
        }

        let inline_len = 2
            + sig
                .required_positional
                .iter()
                .map(|param| param.name.len())
                .sum::<usize>()
            + sig.required_positional.len().saturating_sub(1) * 2;

        inline_len <= self.config.line_length
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Custom completions and shapes
    // ─────────────────────────────────────────────────────────────────────────

    /// Write a custom completion annotation (`@cmd` or `@[items]`).
    pub(super) fn write_custom_completion(&mut self, completion: &Option<Completion>) {
        match completion {
            Some(Completion::Command(decl_id)) => {
                let decl = self.working_set.get_decl(*decl_id);
                let name = decl.name();
                self.write("@");
                if name.contains(' ')
                    || name.contains('-')
                    || name.contains('[')
                    || name.contains(']')
                {
                    self.write("\"");
                    self.write(name);
                    self.write("\"");
                } else {
                    self.write(name);
                }
            }
            Some(Completion::List(list)) => {
                self.write("@[");
                match list {
                    NuCow::Borrowed(items) => {
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                self.write(" ");
                            }
                            self.write(item);
                        }
                    }
                    NuCow::Owned(items) => {
                        for (i, item) in items.iter().enumerate() {
                            if i > 0 {
                                self.write(" ");
                            }
                            self.write(item);
                        }
                    }
                }
                self.write("]");
            }
            None => {}
        }
    }

    /// Write a [`SyntaxShape`], normalising special cases (e.g. `closure()`
    /// → `closure`).
    pub(super) fn write_shape(&mut self, shape: &SyntaxShape) {
        match shape {
            SyntaxShape::Closure(None) => self.write("closure"),
            SyntaxShape::Closure(_) => {
                let rendered = shape.to_string();
                if rendered == "closure()" {
                    self.write("closure");
                } else {
                    self.write(&rendered);
                }
            }
            other => self.write(&other.to_string()),
        }
    }
}
