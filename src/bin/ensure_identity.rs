use fold_db::security::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Structure to hold the persistent node identity
#[derive(Serialize, Deserialize)]
struct NodeIdentity {
    private_key: String,
    public_key: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = Path::new("config");
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    let identity_path = config_dir.join("node_identity.json");

    let identity = if identity_path.exists() {
        let content = fs::read_to_string(&identity_path)?;
        match serde_json::from_str::<NodeIdentity>(&content) {
            Ok(id) => id,
            Err(_) => {
                // If invalid JSON, treat as missing and overwrite
                eprintln!("Invalid identity file found, regenerating...");
                generate_new_identity(&identity_path)?
            }
        }
    } else {
        generate_new_identity(&identity_path)?
    };

    // Print ONLY the public key to stdout so it can be captured by scripts
    print!("{}", identity.public_key);
    Ok(())
}

fn generate_new_identity(path: &Path) -> Result<NodeIdentity, Box<dyn std::error::Error>> {
    let keypair = FoldKeyPair::generate()?;
    let identity = NodeIdentity {
        private_key: keypair.secret_key_base64(),
        public_key: keypair.public_key_base64(),
    };

    let content = serde_json::to_string_pretty(&identity)?;
    fs::write(path, content)?;
    eprintln!("Generated new identity at {:?}", path);
    Ok(identity)
}

// Wrapper to use library key generation
struct FoldKeyPair(Ed25519KeyPair);

impl FoldKeyPair {
    fn generate() -> Result<Self, Box<dyn std::error::Error>> {
        Ed25519KeyPair::generate()
            .map(FoldKeyPair)
            .map_err(|e| format!("Failed to generate keypair: {}", e).into())
    }

    fn secret_key_base64(&self) -> String {
        self.0.secret_key_base64()
    }

    fn public_key_base64(&self) -> String {
        self.0.public_key_base64()
    }
}
