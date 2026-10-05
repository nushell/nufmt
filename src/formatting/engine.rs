//! Engine state initialization and command registration.
//!
//! Sets up the Nushell engine state with built-in commands so the parser
//! can resolve syntax for formatting.

use log::debug;
use nu_protocol::{
    engine::{Call, Command, CommandType as NuCommandType, EngineState, Stack, StateWorkingSet},
    Category, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

/// Stub implementation of the `where` keyword so the parser can resolve it.
#[derive(Clone)]
pub(super) struct WhereKeyword;

impl Command for WhereKeyword {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> Signature {
        Signature::build("where")
            .required(
                "condition",
                SyntaxShape::RowCondition,
                "filter row condition or closure",
            )
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "filter values of an input list based on a condition"
    }

    fn command_type(&self) -> NuCommandType {
        NuCommandType::Keyword
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(input)
    }
}

/// Stub implementation of the `export-env` keyword so the parser can resolve
/// it. The real command lives in `nu-command`, which the formatter does not
/// depend on; without it every `export-env` block parses as garbage and is
/// left unformatted (issue #233).
#[derive(Clone)]
pub(super) struct ExportEnvKeyword;

impl Command for ExportEnvKeyword {
    fn name(&self) -> &str {
        "export-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("export-env")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "block",
                SyntaxShape::Block,
                "The block to run to set the environment.",
            )
            .category(Category::Env)
    }

    fn description(&self) -> &str {
        "Run a block and preserve its environment in a current scope."
    }

    fn command_type(&self) -> NuCommandType {
        NuCommandType::Keyword
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(input)
    }
}

/// Stub implementation of the `with-env` command. The parser looks it up by
/// name to represent the `FOO=bar cmd` shorthand; without it the `FOO=bar`
/// part is left out of the AST and lost when formatting.
#[derive(Clone)]
pub(super) struct WithEnvCommand;

impl Command for WithEnvCommand {
    fn name(&self) -> &str {
        "with-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("with-env")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required(
                "variable",
                SyntaxShape::Any,
                "The environment variable to temporarily set.",
            )
            .required(
                "block",
                SyntaxShape::Closure(None),
                "The block to run once the variable is set.",
            )
            .category(Category::Env)
    }

    fn description(&self) -> &str {
        "Runs a block with an environment variable set."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(input)
    }
}

/// Build the default engine state with `nu-cmd-lang` built-ins plus the
/// formatter's own keyword stubs.
pub(super) fn get_engine_state() -> EngineState {
    let mut engine_state = nu_cmd_lang::create_default_context();
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(WhereKeyword));
        working_set.add_decl(Box::new(ExportEnvKeyword));
        working_set.add_decl(Box::new(WithEnvCommand));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        debug!("failed to merge formatter context: {err:?}");
    }

    engine_state
}
