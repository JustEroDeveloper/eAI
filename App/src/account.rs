use crate::data::base_dir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

/// Repeated hashing makes a stolen accounts file slower to brute-force.
const ITERATIONS: u32 = 100_000;

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    /// Salt for the password hash. Empty when the profile has no password.
    #[serde(default)]
    pub salt: String,
    /// Hex-encoded password hash. `None` means the profile has no password.
    #[serde(default)]
    pub password: Option<String>,
}

impl Account {
    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    pub fn verify(&self, password: &str) -> bool {
        let Some(expected) = self.password.as_deref() else {
            return true;
        };
        hash_password(password, &self.salt) == expected
    }
}

fn accounts_file() -> PathBuf {
    base_dir().join("accounts.json")
}

pub fn load_accounts() -> Vec<Account> {
    std::fs::read_to_string(accounts_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_accounts(accounts: &[Account]) {
    let _ = std::fs::create_dir_all(base_dir());
    if let Ok(json) = serde_json::to_string_pretty(accounts) {
        let path = accounts_file();
        let _ = std::fs::write(&path, json);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

pub fn new_salt() -> String {
    Uuid::new_v4().simple().to_string()
}

pub fn hash_password(password: &str, salt: &str) -> String {
    let mut digest = Sha256::digest(format!("{salt}:{password}").into_bytes());
    for _ in 1..ITERATIONS {
        let mut hasher = Sha256::new();
        hasher.update(digest);
        hasher.update(salt.as_bytes());
        digest = hasher.finalize();
    }
    to_hex(&digest)
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let salt = new_salt();
        let hash = hash_password("correct horse", &salt);
        assert_ne!(hash, "correct horse");
        assert_eq!(hash.len(), 64);
        assert!(hash_password("correct horse", &salt) == hash);
    }

    #[test]
    fn account_checks_password() {
        let salt = new_salt();
        let account = Account {
            id: Uuid::new_v4(),
            name: "Alex".into(),
            salt: salt.clone(),
            password: Some(hash_password("hunter2", &salt)),
        };
        assert!(account.verify("hunter2"));
        assert!(!account.verify("hunter3"));
        assert!(account.has_password());
    }

    #[test]
    fn account_without_password_accepts_anything() {
        let account = Account {
            id: Uuid::new_v4(),
            name: "Sam".into(),
            salt: String::new(),
            password: None,
        };
        assert!(account.verify(""));
        assert!(account.verify("whatever"));
        assert!(!account.has_password());
    }
}
