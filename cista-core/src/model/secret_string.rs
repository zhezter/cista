use secrecy::{CloneableSecret, SerializableSecret, Zeroize};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretString(String);

impl CloneableSecret for SecretString {}

impl Zeroize for SecretString {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl SerializableSecret for SecretString {}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        SecretString(s)
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString([REDACTED])")
    }
}

impl SecretString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
