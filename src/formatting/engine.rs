//! Engine state initialization and command registration.
//!
//! Sets up the Nushell engine state with built-in commands so the parser
//! can resolve syntax for formatting.

use log::debug;
use nu_protocol::{
    engine::{Call, Command, CommandType as NuCommandType, EngineState, Stack, StateWorkingSet},
    Category, PipelineData, ShellError, Signature, SyntaxShape,
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

/// Build the default engine state with `nu-cmd-lang` built-ins plus the
/// formatter's own keyword stubs.
pub(super) fn get_engine_state() -> EngineState {
    let mut engine_state = nu_cmd_lang::create_default_context();
    let delta = {
        let mut working_set = StateWorkingSet::new(&engine_state);
        working_set.add_decl(Box::new(WhereKeyword));
        working_set.render()
    };

    if let Err(err) = engine_state.merge_delta(delta) {
        debug!("failed to merge formatter context: {err:?}");
    }

    engine_state
}
