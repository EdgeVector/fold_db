use crate::commands::CommandOutput;
use crate::error::CliError;
use crate::output::spinner;
use crate::output::OutputMode;
use fold_db::fold_node::OperationProcessor;

pub async fn run(
    query: &str,
    user_hash: &str,
    max_iterations: usize,
    processor: &OperationProcessor,
    mode: OutputMode,
) -> Result<CommandOutput, CliError> {
    let sp = if mode == OutputMode::Human {
        Some(spinner::new_spinner(&format!(
            "Thinking (max {} iterations)...",
            max_iterations
        )))
    } else {
        None
    };

    let result = processor.llm_query(query, user_hash, max_iterations).await;

    if let Some(ref pb) = sp {
        spinner::finish_spinner(pb, "Done");
    }

    let (answer, tool_calls) = result?;
    Ok(CommandOutput::AskAnswer {
        answer,
        tool_calls,
    })
}
