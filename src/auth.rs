use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct CredentialsFile {
    users: HashMap<String, String>,
}

pub fn load_credentials(path: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let creds: CredentialsFile = serde_json::from_str(&contents)?;
    Ok(creds.users)
}
