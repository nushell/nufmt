//! Call and argument formatting.
//!
//! Handles `def`, `let`/`mut`/`const`, `extern`, conditional, and regular
//! command calls, including signature rendering and custom completions.

use super::scan::parens_enclose;
use super::{BranchExpansionKey, CommandType, Formatter};
use crate::config::ConsistentBranches;
use nu_protocol::{
    CollectionColumns, Completion, Signature, Span, SyntaxShape,
    ast::{Argument, Expr, Expression, ExternalArgument},
};
use nu_utils::NuCow;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Commands whose block arguments are formatted specially.
pub(super) const BLOCK_COMMANDS: &[&str] = &[
    "for",
    "while",
    "loop",
    "module",
    "export module",
    "export-env",
];
pub(super) const CONDITIONAL_COMMANDS: &[&str] = &["if", "try"];
pub(super) const DEF_COMMANDS: &[&str] = &["def", "def-env", "export def"];
pub(super) const EXTERN_COMMANDS: &[&str] = &["extern", "export extern"];
pub(super) const ALIAS_COMMANDS: &[&str] = &["alias", "export alias"];
pub(super) const LET_COMMANDS: &[&str] = &["let", "let-env", "mut", "const", "export const"];

// ─────────────────────────────────────────────────────────────────────────────
// Free helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Return the end byte position of an [`Argument`]'s source span.
///
/// For named arguments the end is the value span's end when a value is
/// present, or the flag span's end otherwise.
fn argument_end_pos(arg: &Argument) -> usize {
    match arg {
        Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
            expr.span.end
        }
        Argument::Named(named) => named
            .2
            .as_ref()
            .map_or(named.0.span.end, |value| value.span.end),
    }
}

/// When `arg`, at `index` among the arguments of the `if` or `try` call named
/// `decl_name`, is one of its branches, return the branch body: the block or
/// closure itself, or the expression after `else`/`catch`/`finally`. The
/// condition of an `if` is not a branch.
fn conditional_branch_body<'c>(
    decl_name: &str,
    index: usize,
    arg: &'c Argument,
) -> Option<&'c Expression> {
    let (Argument::Positional(expr) | Argument::Unknown(expr)) = arg else {
        return None;
    };
    if decl_name == "if" && index == 0 {
        return None;
    }
    match &expr.expr {
        Expr::Block(_) | Expr::Closure(_) => Some(expr),
        Expr::Keyword(keyword) => Some(&keyword.expr),
        _ => None,
    }
}

impl<'a> Formatter<'a> {
    // ─────────────────────────────────────────────────────────────────────────
    // Call formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a call expression.
    pub(super) fn format_call(&mut self, call: &nu_protocol::ast::Call) {
        let inherited_branch_expansion = self.pending_branch_expansion.take();
        let decl = self.working_set.get_decl(call.decl_id);
        let decl_name = decl.name();
        let cmd_type = Self::classify_command(decl_name);
        let head_text = self.call_head_text(call);

        if self.should_wrap_call_multiline(call, &cmd_type) {
            self.format_wrapped_call(call);
            return;
        }

        self.write_call_head(call, head_text.as_deref());

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

        // An `else if` inherits the decision made for the whole chain.
        let expand_branches = matches!(cmd_type, CommandType::Conditional)
            && inherited_branch_expansion
                .unwrap_or_else(|| self.cached_conditional_branches_need_expansion(call));

        for (index, arg) in call.arguments.iter().enumerate() {
            if matches!(cmd_type, CommandType::Regular)
                && !self.argument_belongs_to_call_source(call, arg)
            {
                continue;
            }
            // Only a block, a closure, or an `else if` belongs to the chain.
            // Anything else after `else` (e.g. a parenthesized `if`) starts
            // its own chain.
            if matches!(cmd_type, CommandType::Conditional)
                && conditional_branch_body(decl_name, index, arg)
                    .is_some_and(|body| self.continues_conditional_chain(body))
            {
                self.pending_branch_expansion = Some(expand_branches);
            }
            // The match scrutinee needs explicit parens around pipelines such
            // as `($in | describe)`; without them the `|` ends the `match` call.
            let is_match_scrutinee = decl_name == "match"
                && matches!(
                    arg,
                    Argument::Positional(positional) if !matches!(positional.expr, Expr::MatchBlock(_))
                );
            if is_match_scrutinee {
                self.preserve_subexpr_parens_depth += 1;
            }
            self.format_call_argument(arg, &cmd_type);
            self.pending_branch_expansion = None;
            if is_match_scrutinee {
                self.preserve_subexpr_parens_depth -= 1;
            }
        }

        if preserve_not_subexpr_parens {
            self.preserve_subexpr_parens_depth -= 1;
        }
    }

    /// Like [`Self::conditional_branches_need_expansion`], but remembers the
    /// answer for each chain. Rendering a branch to measure it formats the
    /// chains nested inside it, so without the cache the work would double
    /// with every level of nesting.
    ///
    /// The answer depends on the available width and on whether parens are
    /// kept, so it is only reused in the same indent, multiline-pipeline,
    /// and paren-preserving context; probes that render at indent 0 must not
    /// decide for the real pass.
    fn cached_conditional_branches_need_expansion(&self, call: &nu_protocol::ast::Call) -> bool {
        let key = BranchExpansionKey {
            head_start: call.head.start,
            indent_level: self.indent_level,
            force_pipeline_multiline: self.force_pipeline_multiline_depth > 0,
            preserve_subexpr_parens: self.preserve_subexpr_parens_depth > 0,
        };
        let cached = self.branch_expansion_cache.borrow().get(&key).copied();
        if let Some(expand) = cached {
            return expand;
        }

        let expand = self.conditional_branches_need_expansion(call);
        self.branch_expansion_cache.borrow_mut().insert(key, expand);
        expand
    }

    /// Return `true` when `body` is part of an `if`/`try` chain: a block, a
    /// closure, or a nested `if`/`try` call (an `else if`).
    fn continues_conditional_chain(&self, body: &Expression) -> bool {
        match &body.expr {
            Expr::Block(_) | Expr::Closure(_) => true,
            Expr::Call(inner) => {
                CONDITIONAL_COMMANDS.contains(&self.working_set.get_decl(inner.decl_id).name())
            }
            _ => false,
        }
    }

    /// Decide whether every branch of the `if`/`try` chain headed by `call`
    /// must be expanded because one of them spans several lines (issue #217).
    fn conditional_branches_need_expansion(&self, call: &nu_protocol::ast::Call) -> bool {
        if self.config.consistent_branches == ConsistentBranches::Never {
            return false;
        }

        let mut branches = Vec::new();
        self.collect_conditional_branches(call, &mut branches);
        if branches.len() < 2 {
            return false;
        }

        if self.config.consistent_branches == ConsistentBranches::SingleLine {
            let written_on_one_line = self
                .source
                .get(call.head.start..call.span().end)
                .is_some_and(|text| !text.contains(&b'\n'));
            // A chain written across lines keeps its layout. When every
            // branch with a body is already expanded, as a previous run
            // leaves an expanded chain, a short `catch {|e| ... }` closure
            // stays expanded too instead of collapsing back.
            if !written_on_one_line {
                return self.every_branch_written_expanded(&branches);
            }
        }

        branches
            .iter()
            .any(|branch| self.branch_renders_multiline(branch))
    }

    /// Return `true` when every branch with a body is written across
    /// several lines, and at least one branch has a body. Empty branches
    /// (`{ }`) keep their layout either way, so they don't count.
    fn every_branch_written_expanded(&self, branches: &[&Expression]) -> bool {
        let mut any_body = false;
        for branch in branches {
            let (Expr::Block(block_id) | Expr::Closure(block_id)) = &branch.expr else {
                continue;
            };
            if self.working_set.get_block(*block_id).pipelines.is_empty() {
                continue;
            }
            any_body = true;
            let written_expanded = self
                .source
                .get(branch.span.start..branch.span.end)
                .is_some_and(|text| text.contains(&b'\n'));
            if !written_expanded {
                return false;
            }
        }
        any_body
    }

    /// Collect the block and closure branches of an `if`/`try` chain,
    /// following `else if`.
    fn collect_conditional_branches<'c>(
        &self,
        call: &'c nu_protocol::ast::Call,
        branches: &mut Vec<&'c Expression>,
    ) {
        let decl_name = self.working_set.get_decl(call.decl_id).name();
        for (index, arg) in call.arguments.iter().enumerate() {
            let Some(body) = conditional_branch_body(decl_name, index, arg) else {
                continue;
            };
            match &body.expr {
                Expr::Block(_) | Expr::Closure(_) => branches.push(body),
                Expr::Call(inner)
                    if CONDITIONAL_COMMANDS
                        .contains(&self.working_set.get_decl(inner.decl_id).name()) =>
                {
                    self.collect_conditional_branches(inner.as_ref(), branches);
                }
                _ => {}
            }
        }
    }

    /// Return `true` when a block or closure branch will be laid out across
    /// several lines.
    fn branch_renders_multiline(&self, branch: &Expression) -> bool {
        let (Expr::Block(block_id) | Expr::Closure(block_id)) = &branch.expr else {
            return false;
        };

        // Decide from the AST when possible, mirroring the layout rules of
        // `format_block_expression` and `format_closure_expression`. Only a
        // body that might fit on one line is rendered to find out.
        let block = self.working_set.get_block(*block_id);
        // An empty branch keeps its layout (`catch {|e|}` included), so it
        // doesn't call for expanding the others.
        if block.pipelines.is_empty()
            && !self.has_comments_in_span(branch.span.start, branch.span.end)
        {
            return false;
        }
        let single_simple_element = self.has_single_statement(block)
            && block.pipelines[0].elements.len() == 1
            && !self.block_has_nested_structures(block);
        let certainly_multiline = if self.closure_span_has_params(branch.span) {
            // `{|x| ...}` keeps a simple body inline even across lines.
            matches!(branch.expr, Expr::Closure(_)) && !single_simple_element
        } else {
            let written_multiline = self
                .source
                .get(branch.span.start..branch.span.end)
                .is_some_and(|text| text.contains(&b'\n'));
            !block.pipelines.is_empty() && (written_multiline || !single_simple_element)
        };
        if certainly_multiline {
            return true;
        }

        // Render the branch in the same context the real pass uses for it.
        // By the time the real pass reaches the branch it has written the
        // condition and its comments, so the probe starts at the branch.
        let indent_level = self.indent_level;
        let conditional_context_depth = self.conditional_context_depth + 1;
        let preserve_subexpr_parens_depth = self.preserve_subexpr_parens_depth;
        let force_pipeline_multiline_depth = self.force_pipeline_multiline_depth;
        let inline_comment_upper_bound = self.inline_comment_upper_bound;
        self.probe_format(|probe| {
            probe.last_pos = probe.last_pos.max(branch.span.start);
            probe.indent_level = indent_level;
            probe.conditional_context_depth = conditional_context_depth;
            probe.preserve_subexpr_parens_depth = preserve_subexpr_parens_depth;
            probe.force_pipeline_multiline_depth = force_pipeline_multiline_depth;
            probe.inline_comment_upper_bound = inline_comment_upper_bound;
            probe.format_block_or_expr(branch);
        })
        .contains(&b'\n')
    }

    /// Return `true` when rebuilding `call` (whose expression spans
    /// `expr_span`) from the AST would delete source text the parser left out
    /// of it.
    ///
    /// Two cases: `export-env` keeps only its first argument (the `extra` in
    /// `export-env {} extra`), and a flag the command doesn't have is not in
    /// the AST at all (`do -p { ... }` after `-p` was removed in nushell
    /// 0.116).
    pub(super) fn call_drops_source(&self, call: &nu_protocol::ast::Call, expr_span: Span) -> bool {
        self.export_env_call_drops_source(call, expr_span.end)
            || self.call_has_unknown_flag(call, expr_span)
    }

    /// Return `true` when `call` is an `export-env` call with text after its
    /// first argument.
    fn export_env_call_drops_source(&self, call: &nu_protocol::ast::Call, expr_end: usize) -> bool {
        if self.working_set.get_decl(call.decl_id).name() != "export-env" {
            return false;
        }

        let parsed_end = call.span().end;

        parsed_end < expr_end
            && expr_end <= self.source.len()
            && self.source[parsed_end..expr_end]
                .iter()
                .any(|byte| !byte.is_ascii_whitespace())
    }

    /// Return `true` when the parser reported an unknown flag that belongs to
    /// `call` itself, not to a call nested in one of its arguments.
    fn call_has_unknown_flag(&self, call: &nu_protocol::ast::Call, expr_span: Span) -> bool {
        self.unknown_flags_in(call.head.end, expr_span.end)
            .any(|flag| {
                !call
                    .arguments
                    .iter()
                    .any(|arg| arg.span().contains_span(*flag))
            })
    }

    /// Format a call to `with-env`, which nufmt knows only so that the
    /// parser keeps the `FOO=bar cmd` shorthand. Return `false` for any other
    /// call.
    ///
    /// The parser represents the shorthand as a `with-env` call without a
    /// head, written here as the variables (as authored) and then the
    /// command. An explicit `with-env {..} {..}` call keeps its braced
    /// arguments as authored, as it did when the command was unknown.
    pub(super) fn try_format_with_env_call(&mut self, call: &nu_protocol::ast::Call) -> bool {
        if self.working_set.get_decl(call.decl_id).name() != "with-env" {
            return false;
        }
        if call.head == Span::unknown() {
            return self.try_format_env_shorthand(call);
        }

        self.write_call_head(call, None);
        for arg in &call.arguments {
            if !self.argument_belongs_to_call_source(call, arg) {
                continue;
            }
            match arg {
                Argument::Positional(expr) | Argument::Unknown(expr)
                    if self.source.get(expr.span.start) == Some(&b'{') =>
                {
                    self.space();
                    self.write_braced_external_argument(expr);
                }
                _ => self.format_call_argument(arg, &CommandType::Regular),
            }
        }
        true
    }

    /// Format `FOO=bar cmd`: the variables as written, then the command.
    fn try_format_env_shorthand(&mut self, call: &nu_protocol::ast::Call) -> bool {
        let [Argument::Positional(variables), Argument::Positional(body)] =
            call.arguments.as_slice()
        else {
            return false;
        };
        let Expr::Closure(block_id) = body.expr else {
            return false;
        };
        let block = self.working_set.get_block(block_id);
        let [pipeline] = block.pipelines.as_slice() else {
            return false;
        };
        let [element] = pipeline.elements.as_slice() else {
            return false;
        };

        self.write_span(variables.span);
        self.space();
        self.format_expression(&element.expr);
        true
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

        // Measure the formatted call, not the source, so a second pass decides the same (#242).
        let head_text = self.call_head_text(call);
        let indent_level = self.indent_level;
        let rendered = self.probe_format(|probe| {
            probe.indent_level = indent_level;
            probe.write_call_head(call, head_text.as_deref());
            for arg in &source_args {
                probe.format_call_argument(arg, &CommandType::Regular);
            }
        });
        if rendered.contains(&b'\n') {
            return false;
        }

        rendered.len() + (self.config.indent * self.indent_level) > self.config.line_length
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

        let head_text = self.call_head_text(call);

        self.write("(");
        self.write_call_head(call, head_text.as_deref());
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

    /// Write the command name of `call`, whose source text is `head_text`,
    /// with a leading `%` kept and the spacing of a multi-word command
    /// (`export   def`) normalized.
    fn write_call_head(&mut self, call: &nu_protocol::ast::Call, head_text: Option<&str>) {
        if call.head.end == 0 {
            return;
        }
        self.write_prefer_builtin_sigil(call.head.start);
        match head_text {
            Some(head) if head.contains(' ') && head.split_whitespace().count() > 1 => {
                let normalized = head.split_whitespace().collect::<Vec<_>>().join(" ");
                self.write(&normalized);
            }
            _ => self.write_span(call.head),
        }
    }

    /// Write the `%` that forces a built-in command (`%ls`) when it sits
    /// right before the command head at `head_start`. Since nushell 0.116
    /// the head span no longer includes it.
    fn write_prefer_builtin_sigil(&mut self, head_start: usize) {
        if head_start > 0 && self.source.get(head_start - 1) == Some(&b'%') {
            self.write("%");
        }
    }

    /// Return the raw source text at the call's head span, or `None` if the
    /// span is invalid.
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

    /// Return `true` if the call's head text is an alias-declaration keyword
    /// (e.g. `alias` or `export alias`).
    fn call_head_matches_alias_decl_command(&self, call: &nu_protocol::ast::Call) -> bool {
        self.call_head_text(call)
            .as_deref()
            .is_some_and(|head| ALIAS_COMMANDS.contains(&head))
    }

    /// Format an invocation of an alias (i.e. a call *using* an alias rather
    /// than declaring one).  Preserves the exact source text after the head.
    fn format_alias_invocation_call(&mut self, call: &nu_protocol::ast::Call) {
        // Only consider arguments that belong to the authored source (not
        // parser-injected alias expansion arguments – issue #180).
        let call_end = call
            .arguments
            .iter()
            .filter(|arg| self.argument_belongs_to_call_source(call, arg))
            .map(argument_end_pos)
            .max()
            .unwrap_or(call.head.end)
            .min(self.source.len());

        if call.head.end < call_end {
            self.write_bytes(&self.source[call.head.end..call_end]);
        }
    }

    /// Return `true` if `arg`'s source span starts at or after the call head,
    /// meaning the argument was written by the user rather than injected by the
    /// parser (e.g. default values or alias expansions).
    fn argument_belongs_to_call_source(
        &self,
        call: &nu_protocol::ast::Call,
        arg: &Argument,
    ) -> bool {
        let (span_start, span_end) = match arg {
            Argument::Positional(expr) | Argument::Unknown(expr) | Argument::Spread(expr) => {
                (expr.span.start, expr.span.end)
            }
            Argument::Named(named) => (
                named.0.span.start,
                named
                    .2
                    .as_ref()
                    .map_or(named.0.span.end, |value| value.span.end),
            ),
        };

        // Exclude parser-injected arguments that don't map to source bytes.
        span_end > span_start && span_start >= call.head.start
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

        let rhs_end = call.arguments.iter().map(argument_end_pos).max();

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

    /// Format a slice of arguments using the `Regular` command strategy.
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
    /// Format a positional argument, choosing a strategy based on `cmd_type`.
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
    /// Format an argument for `def` commands (name string, signature, or body).
    fn format_def_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::String(_) => self.format_expression(positional),
            Expr::Signature(sig) => {
                if self.has_comments_in_span(positional.span.start, positional.span.end) {
                    self.write_expr_span(positional);
                } else {
                    self.format_signature(sig, positional.span);
                }
            }
            Expr::Closure(block_id) | Expr::Block(block_id) => {
                self.format_block_expression(*block_id, positional.span, true);
            }
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for `extern` commands (preserve original signature).
    /// Format an argument for `extern` commands, preserving the raw source
    /// for signature nodes so that extern declarations stay idempotent.
    fn format_extern_argument(&mut self, positional: &Expression) {
        match &positional.expr {
            Expr::Signature(_) => self.write_expr_span(positional),
            _ => self.format_expression(positional),
        }
    }

    /// Format an argument for `let`/`mut`/`const` commands.
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
                && let Expr::Subexpression(inner_id) = &pipeline.elements[0].expr.expr
            {
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
        self.write_prefer_builtin_sigil(head.span.start);
        self.format_expression(head);

        let has_injected_prefix_args = args.iter().any(|arg| match arg {
            ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) => {
                expr.span.end > expr.span.start && expr.span.end <= head.span.start
            }
        });

        if has_injected_prefix_args {
            let authored_tail_end = args
                .iter()
                .filter_map(|arg| match arg {
                    ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr)
                        if expr.span.end > expr.span.start
                            && expr.span.start >= head.span.start =>
                    {
                        Some(expr.span.end)
                    }
                    _ => None,
                })
                .max()
                .unwrap_or(head.span.end)
                .min(self.source.len());

            if head.span.end < authored_tail_end {
                self.write_bytes(&self.source[head.span.end..authored_tail_end]);
            }
            return;
        }

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
                ExternalArgument::Regular(arg_expr)
                    if self.source.get(arg_expr.span.start) == Some(&b'{') =>
                {
                    self.write_braced_external_argument(arg_expr);
                }
                ExternalArgument::Regular(arg_expr) => self.format_expression(arg_expr),
                ExternalArgument::Spread(spread_expr)
                    if self.source.get(spread_expr.span.start) == Some(&b'{') =>
                {
                    self.write("...");
                    self.write_braced_external_argument(spread_expr);
                }
                ExternalArgument::Spread(spread_expr) => {
                    self.write("...");
                    self.format_expression(spread_expr);
                }
            }
        }
    }

    /// Write a `{...}` argument of an external call as authored, except that
    /// an empty `{ }` becomes `{}`.
    ///
    /// Commands nufmt doesn't know (`each`, `where`, `merge`, ...) parse as
    /// external calls. Before nushell 0.116 their braced arguments came back
    /// as raw text; now they are closures, blocks, and records. Writing them
    /// as authored keeps the dependency upgrade from reformatting them.
    fn write_braced_external_argument(&mut self, arg_expr: &Expression) {
        let raw = &self.source[arg_expr.span.start..arg_expr.span.end];
        if raw.len() >= 2
            && raw.ends_with(b"}")
            && raw[1..raw.len() - 1].iter().all(u8::is_ascii_whitespace)
        {
            self.write("{}");
        } else {
            self.write_expr_span(arg_expr);
        }
    }

    /// Return `true` if the external call's source tail ends with exactly the
    /// same token sequence as the last N parsed `args`, which indicates that the
    /// parser prepended alias-expanded tokens before the user-written suffix.
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

    /// Split `bytes` into whitespace-delimited word tokens, honouring
    /// single- and double-quoted strings.
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

    /// Attempt to emit `rhs` without its outer parentheses when they are
    /// redundant around a single pipeline (e.g. `let x = (a | b)` → `let x = a | b`).
    /// Returns `true` if the rewrite was applied.
    fn try_write_redundant_parenthesized_pipeline_rhs(&mut self, rhs: &Expression) -> bool {
        let raw = self.get_span_content(rhs.span);
        let trimmed = raw.trim_ascii();
        if trimmed.len() < 3 || trimmed.contains(&b'\n') {
            return false;
        }

        // The outer parens must wrap the whole RHS: `(a | b) + (c | d)`
        // starts and ends with a paren but needs both pairs.
        if !(trimmed.contains(&b'|') && parens_enclose(trimmed)) {
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

    /// Attempt to emit an empty block/closure argument as `{}` rather than
    /// going through the normal block formatter.  Returns `true` on success.
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

    /// Attempt to normalise a span that looks like a closure (`{ |p| body }`).
    /// Returns `true` if the span was handled, `false` if the caller should
    /// fall back to the regular expression formatter.
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

        self.write_single_line_closure(inner)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Signature formatting
    // ─────────────────────────────────────────────────────────────────────────

    /// Format a parameter signature (`[x: int, --flag(-f)]`).
    ///
    /// `sig_span` is the original `[…]` source span; defaults are recovered
    /// from that text when the parser drops or expands them (issue #204).
    pub(super) fn format_signature(&mut self, sig: &Signature, sig_span: Span) {
        let args_from_source = self.signature_args_from_source(sig_span);

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

            let arg_src = args_from_source.iter().find(|arg| {
                if let Some(span) = arg.name {
                    token_matches_param(&self.source[span.start..span.end], &param.name, false)
                } else {
                    false
                }
            });

            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&param.shape);

                if let Some(completion_span) = arg_src
                    .and_then(|a| a.types.first())
                    .and_then(|t| t.completion)
                {
                    self.write("@");
                    self.write_bytes(&self.source[completion_span.start..completion_span.end]);
                } else {
                    self.write_custom_completion(&param.completion);
                }
            }
            // Rare: parser may still leave a default only in source.
            if let Some(default_span) = arg_src.and_then(|a| a.default_values.first()) {
                self.write(" = ");
                self.write_bytes(&self.source[default_span.start..default_span.end]);
            }
        }

        // Optional positional parameters
        for param in &sig.optional_positional {
            write_sep(self, &mut first, has_multiline);
            self.write(&param.name);

            let arg_src = args_from_source.iter().find(|arg| {
                if let Some(span) = arg.name {
                    token_matches_param(&self.source[span.start..span.end], &param.name, false)
                } else {
                    false
                }
            });
            let source_default = arg_src.and_then(|a| a.default_values.first());

            // Emit `?` only when there is truly no default (AST or source).
            // Unresolvable defaults like `$nu.history-path` become
            // `default_value: None` in the AST but remain in source (issue #204).
            if param.default_value.is_none() && source_default.is_none() {
                self.write("?");
            }
            if param.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&param.shape);

                if let Some(completion_span) = arg_src
                    .and_then(|a| a.types.first())
                    .and_then(|t| t.completion)
                {
                    self.write("@");
                    self.write_bytes(&self.source[completion_span.start..completion_span.end]);
                } else {
                    self.write_custom_completion(&param.completion);
                }
            }
            if let Some(span) = source_default {
                self.write(" = ");
                self.write_bytes(&self.source[span.start..span.end]);
            } else if let Some(default) = &param.default_value {
                self.write(" = ");
                // Use raw source to preserve original quote style (issue #179).
                let span = default.span();
                if span.start < span.end && span.end <= self.source.len() {
                    self.write_bytes(&self.source[span.start..span.end]);
                } else {
                    self.write(&default.to_parsable_string(" ", &nu_protocol::Config::default()));
                }
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

            let flag_name = if flag.long.is_empty() {
                flag.short.map(|c| c.to_string()).unwrap_or_default()
            } else {
                flag.long.clone()
            };

            let arg_src = args_from_source.iter().find(|arg| {
                if let Some(span) = arg.name {
                    token_matches_param(&self.source[span.start..span.end], &flag_name, true)
                } else {
                    false
                }
            });

            if let Some(shape) = &flag.arg {
                self.write(": ");
                self.write_shape(shape);

                if let Some(completion_span) = arg_src
                    .and_then(|a| a.types.first())
                    .and_then(|t| t.completion)
                {
                    self.write("@");
                    self.write_bytes(&self.source[completion_span.start..completion_span.end]);
                } else {
                    self.write_custom_completion(&flag.completion);
                }
            }

            let source_default = arg_src.and_then(|a| a.default_values.first());

            if let Some(span) = source_default {
                self.write(" = ");
                self.write_bytes(&self.source[span.start..span.end]);
            } else if let Some(default) = &flag.default_value {
                self.write(" = ");
                // Use raw source to preserve original quote style (issue #179).
                let span = default.span();
                if span.start < span.end && span.end <= self.source.len() {
                    self.write_bytes(&self.source[span.start..span.end]);
                } else {
                    self.write(&default.to_parsable_string(" ", &nu_protocol::Config::default()));
                }
            }
        }

        // Rest positional (last)
        if let Some(rest) = &sig.rest_positional {
            write_sep(self, &mut first, has_multiline);
            self.write("...");
            self.write(&rest.name);

            let arg_src = args_from_source.iter().find(|arg| {
                if let Some(span) = arg.name {
                    token_matches_param(&self.source[span.start..span.end], &rest.name, false)
                } else {
                    false
                }
            });

            if rest.shape != SyntaxShape::Any {
                self.write(": ");
                self.write_shape(&rest.shape);

                if let Some(completion_span) = arg_src
                    .and_then(|a| a.types.first())
                    .and_then(|t| t.completion)
                {
                    self.write("@");
                    self.write_bytes(&self.source[completion_span.start..completion_span.end]);
                } else {
                    self.write_custom_completion(&rest.completion);
                }
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

        inline_len + (self.config.indent * self.indent_level) <= self.config.line_length
    }

    pub fn signature_args_from_source(&self, sig_span: Span) -> Vec<ArgFromSource> {
        if sig_span.end <= sig_span.start || sig_span.end > self.source.len() {
            return vec![];
        }

        let body = &self.source[sig_span.start..sig_span.end];
        // With input/output types the parser extends the signature span over
        // `: in -> out`, so take only the leading `[...]` token, the same way
        // `parse_full_signature` does (issue #236).
        let (tokens, _) = nu_parser::lex(body, sig_span.start, &[], &[], true);
        let params_span = tokens.first().map_or(sig_span, |token| token.span);
        let mut params = &self.source[params_span.start..params_span.end];
        if params.len() > 1 && params.ends_with(b":") {
            params = &params[..params.len() - 1];
        }

        // Strip surrounding `[` `]` (or `(` `)`) if present.
        let is_delimited = params.len() >= 2
            && matches!(
                (params.first(), params.last()),
                (Some(b'['), Some(b']')) | (Some(b'('), Some(b')'))
            );
        let (inner, inner_start) = if is_delimited {
            (&params[1..params.len() - 1], params_span.start + 1)
        } else {
            (params, params_span.start)
        };

        let (tokens, _) = nu_parser::lex_signature(inner, inner_start, b"\n\r", b",:=", false);

        // Mirrors the `nu_parser::parse_signatures::parse_signature_helper` state machine.
        enum Mode {
            Arg,
            Type,
            AfterType,
            DefaultValue,
        }
        let mut mode = Mode::Arg;

        let mut args: Vec<ArgFromSource> = Vec::new();
        let mut current_arg = ArgFromSource::default();
        let mut has_data = false;

        for token in &tokens {
            if token.contents == nu_parser::TokenContents::Comment {
                continue;
            }

            let bytes = &self.source[token.span.start..token.span.end];
            match bytes {
                b":" => {
                    if matches!(mode, Mode::AfterType) && has_data {
                        args.push(std::mem::take(&mut current_arg));
                        has_data = false;
                    }
                    mode = Mode::Type;
                }
                b"=" => mode = Mode::DefaultValue,
                b"," => {
                    if has_data {
                        args.push(std::mem::take(&mut current_arg));
                        has_data = false;
                    }
                    mode = Mode::Arg;
                }
                // `--namespace (-n)`: the short form of the flag just named,
                // not a new parameter.
                _ if matches!(mode, Mode::Arg)
                    && bytes.starts_with(b"(-")
                    && current_arg.name.is_some_and(|name| {
                        self.source[name.start..name.end].starts_with(b"--")
                    }) => {}
                _ => match mode {
                    Mode::Arg | Mode::AfterType => {
                        if has_data {
                            args.push(std::mem::take(&mut current_arg));
                        }
                        current_arg.name = Some(token.span);
                        has_data = true;
                        mode = Mode::Arg;
                    }
                    Mode::Type => {
                        let _type = if let Some(i) = bytes.iter().position(|&b| b == b'@') {
                            TypeFromSource {
                                name: Span::new(token.span.start, token.span.start + i),
                                completion: Some(Span::new(
                                    token.span.start + i + 1,
                                    token.span.end,
                                )),
                            }
                        } else {
                            TypeFromSource {
                                name: token.span,
                                completion: None,
                            }
                        };

                        current_arg.types.push(_type);
                        has_data = true;
                        mode = Mode::AfterType;
                    }
                    Mode::DefaultValue => {
                        current_arg.default_values.push(token.span);
                        has_data = true;
                        mode = Mode::AfterType;
                    }
                },
            }
        }

        if has_data {
            args.push(current_arg);
        }

        args
    }
}

#[derive(Default)]
pub struct ArgFromSource {
    name: Option<Span>,
    types: Vec<TypeFromSource>,
    default_values: Vec<Span>,
}

#[allow(dead_code)]
pub struct TypeFromSource {
    name: Span,
    completion: Option<Span>,
}

/// Check if a token matches the given parameter name
fn token_matches_param(token: &[u8], name: &str, is_flag: bool) -> bool {
    if is_flag {
        // keep only the name, dropping any short flag attached to it (e.g. `--name(-x)`)
        let head = match token.iter().position(|&b| b == b'(') {
            Some(pos) => &token[..pos],
            None => token,
        };

        let long = format!("--{name}");
        let short = format!("-{name}");
        head == long.as_bytes() || head == short.as_bytes()
    } else {
        // `...rest` and `optional?` name the parameter without the markers.
        let head = token.strip_prefix(b"...").unwrap_or(token);
        let head = head.strip_suffix(b"?").unwrap_or(head);
        head == name.as_bytes()
    }
}

impl<'a> Formatter<'a> {
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
            // Engine-provided completions of built-in commands have no
            // source syntax, so a user signature never carries one.
            Some(Completion::Builtin(_)) | None => {}
        }
    }

    /// Write a [`SyntaxShape`], normalising special cases (e.g. `closure()`
    /// → `closure`).
    pub(super) fn write_shape(&mut self, shape: &SyntaxShape) {
        self.write(&render_shape(shape));
    }
}

/// Render a syntax shape the way a signature spells it.
///
/// `SyntaxShape`'s `Display` mostly does, but it writes `closure()` for a
/// closure without parameter types, `external-argument` for `external_arg`
/// (which the parser rejects), and record and table column names without
/// the quotes some of them need. Column names are never rewritten, so a
/// column called `external-argument` keeps its name.
fn render_shape(shape: &SyntaxShape) -> String {
    let join = |shapes: &[SyntaxShape]| {
        shapes
            .iter()
            .map(render_shape)
            .collect::<Vec<_>>()
            .join(", ")
    };
    match shape {
        SyntaxShape::ExternalArgument => "external_arg".to_string(),
        SyntaxShape::Closure(None) => "closure".to_string(),
        SyntaxShape::Closure(Some(args)) if args.is_empty() => "closure".to_string(),
        SyntaxShape::Closure(Some(args)) => format!("closure({})", join(args)),
        SyntaxShape::List(item) => format!("list<{}>", render_shape(item)),
        SyntaxShape::OneOf(shapes) if !shapes.is_empty() => format!("oneof<{}>", join(shapes)),
        SyntaxShape::Record(columns) => format!("record{}", render_columns(columns)),
        SyntaxShape::Table(columns) => format!("table{}", render_columns(columns)),
        other => other.to_string(),
    }
}

/// Render the `<name: shape, ...>` columns of a record or table shape.
fn render_columns(columns: &CollectionColumns<SyntaxShape>) -> String {
    if columns.is_empty() {
        return String::new();
    }
    let fields = columns
        .iter()
        .map(|(name, shape)| format!("{}: {}", render_column_name(name), render_shape(shape)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{fields}>")
}

/// Quote a record or table column name when it would not read back as one
/// word: `record<a b: int>` means columns `a` and `b`, not `"a b"`.
fn render_column_name(name: &str) -> String {
    let needs_quotes = name.is_empty()
        || name.chars().any(|c| {
            c.is_whitespace()
                || matches!(
                    c,
                    ':' | ','
                        | '<'
                        | '>'
                        | '"'
                        | '\''
                        | '`'
                        | '#'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '|'
                        | ';'
                        | '\\'
                )
        });
    if needs_quotes {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        name.to_string()
    }
}
