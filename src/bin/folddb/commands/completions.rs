use crate::cli::Cli;
use crate::commands::CommandOutput;
use crate::error::CliError;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

pub fn run(shell: Shell, _verbose: bool) -> Result<CommandOutput, CliError> {
    let mut buf = Vec::new();
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "folddb", &mut buf);
    let script =
        String::from_utf8(buf).map_err(|e| CliError::new(format!("UTF-8 error: {}", e)))?;
    Ok(CommandOutput::Completions(script))
}
