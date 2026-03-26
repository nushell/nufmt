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
pub(super) const LET_COMMANDS: &[&str] = &["let", "let-env", "mut", "const"];

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Call formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a call expression.
    pub(super) fn format_call(&mut self, call: &nu_protocol::ast::Call) {
        let decl = self.working_set.get_decl(call.decl_id);
        let decl_name = decl.name();
        let cmd_type = Self::classify_command(decl_name);

        // Write command name
        if call.head.end != 0 {
            self.write_span(call.head);
        }

        if matches!(cmd_type, CommandType::Let) {
            self.format_let_call(call);
            return;
        }

        for arg in &call.arguments {
            self.format_call_argument(arg, &cmd_type);
        }
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
                Expr::Block(block_id) | Expr::Subexpression(block_id) => {
                    let block = self.working_set.get_block(*block_id);
                    self.format_block(block);
                }
                _ => self.format_expression(rhs),
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

    /// Classify a command name into a [`CommandType`] for formatting purposes.
    pub(super) fn classify_command(name: &str) -> CommandType {
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
            CommandType::Conditional => {
                self.conditional_context_depth += 1;
                self.format_block_or_expr(positional);
                self.conditional_context_depth -= 1;
            }
            CommandType::Block => {
                self.format_block_or_expr(positional);
            }
            CommandType::Let => self.format_let_argument(positional),
            CommandType::Regular => self.format_expression(positional),
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
                self.format_subexpression(*block_id, positional.span);
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

    /// Format an external call (e.g. `^git status`).
    pub(super) fn format_external_call(&mut self, head: &Expression, args: &[ExternalArgument]) {
        // Preserve explicit `^` prefix
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
        let has_multiline = param_count > 3;

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
