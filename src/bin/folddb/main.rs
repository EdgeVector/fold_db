mod cli;
mod commands;
mod error;
mod output;

use clap::Parser;
use cli::Cli;
use error::CliError;
use fold_db::{
    datafold_node::{load_node_config, DataFoldNode, OperationProcessor},
    storage::DatabaseConfig,
};
use output::OutputMode;

fn user_hash_from_pubkey(pubkey: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(pubkey.as_bytes());
    digest[..16]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json_mode = cli.json;
    let mode = if json_mode {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    let config_path = cli.config.clone();

    let mut config = match load_node_config(config_path.as_deref(), None) {
        Ok(c) => c,
        Err(e) => {
            CliError::new(format!("Failed to load config: {}", e))
                .with_hint("Check NODE_CONFIG env var or pass --config <path>")
                .exit(json_mode);
        }
    };

    if let Some(path) = &cli.data_path {
        config.database = DatabaseConfig::Local {
            path: path.clone(),
        };
    }
    if let Some(url) = &cli.schema_service_url {
        config.schema_service_url = Some(url.clone());
    }

    let node = match DataFoldNode::new(config).await {
        Ok(n) => n,
        Err(e) => {
            CliError::new(format!("Failed to create node: {}", e)).exit(json_mode);
        }
    };

    let user_hash = cli
        .user_hash
        .clone()
        .or_else(|| std::env::var("DATAFOLD_USER_HASH").ok())
        .unwrap_or_else(|| user_hash_from_pubkey(node.get_node_public_key()));

    let processor = OperationProcessor::new(node);

    match commands::dispatch(
        &cli.command,
        &processor,
        &user_hash,
        mode,
        config_path.as_deref(),
        cli.verbose,
    )
    .await
    {
        Ok(result) => output::render(&result, mode),
        Err(e) => e.exit(json_mode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_hash_derivation() {
        let hash = user_hash_from_pubkey("test_public_key");
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, user_hash_from_pubkey("test_public_key"));
    }

    #[test]
    fn user_hash_deterministic() {
        let h1 = user_hash_from_pubkey("key_a");
        let h2 = user_hash_from_pubkey("key_b");
        assert_ne!(h1, h2);
    }
}
