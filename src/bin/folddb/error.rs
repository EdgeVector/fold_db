use console::style;
use std::fmt;
use std::process;

pub struct CliError {
    pub message: String,
    pub hint: Option<String>,
    pub cause: Option<String>,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            hint: None,
            cause: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    pub fn exit(self, json_mode: bool) -> ! {
        if json_mode {
            let mut obj = serde_json::json!({
                "ok": false,
                "error": self.message,
            });
            if let Some(hint) = &self.hint {
                obj["hint"] = serde_json::json!(hint);
            }
            println!(
                "{}",
                serde_json::to_string(&obj).unwrap_or_else(|_| format!(
                    "{{\"ok\":false,\"error\":\"{}\"}}",
                    self.message
                ))
            );
        } else {
            eprintln!("{} {}", style("error:").red().bold(), self.message);
            if let Some(cause) = &self.cause {
                eprintln!("  {} {}", style("cause:").yellow(), cause);
            }
            if let Some(hint) = &self.hint {
                eprintln!("  {} {}", style("hint:").cyan(), hint);
            }
        }
        process::exit(1);
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<fold_db::FoldDbError> for CliError {
    fn from(e: fold_db::FoldDbError) -> Self {
        let msg = e.to_string();
        // Extract the root cause from nested errors when available
        let source = std::error::Error::source(&e);
        match source {
            Some(cause) => CliError::new(msg).with_cause(cause.to_string()),
            None => CliError::new(msg),
        }
    }
}

impl From<String> for CliError {
    fn from(s: String) -> Self {
        CliError::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = CliError::new("something failed");
        assert_eq!(err.to_string(), "something failed");
    }

    #[test]
    fn error_with_hint() {
        let err = CliError::new("not found").with_hint("try schema list");
        assert_eq!(err.hint.as_deref(), Some("try schema list"));
    }

    #[test]
    fn error_with_cause() {
        let err = CliError::new("failed").with_cause("network timeout");
        assert_eq!(err.cause.as_deref(), Some("network timeout"));
    }

    #[test]
    fn error_from_string() {
        let err: CliError = "test error".to_string().into();
        assert_eq!(err.message, "test error");
    }
}
