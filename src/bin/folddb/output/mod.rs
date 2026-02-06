pub mod human;
pub mod json;
pub mod spinner;

use crate::commands::CommandOutput;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
}

pub fn render(output: &CommandOutput, mode: OutputMode) {
    match mode {
        OutputMode::Human => human::render(output),
        OutputMode::Json => json::render(output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_mode_equality() {
        assert_eq!(OutputMode::Human, OutputMode::Human);
        assert_eq!(OutputMode::Json, OutputMode::Json);
        assert_ne!(OutputMode::Human, OutputMode::Json);
    }
}
