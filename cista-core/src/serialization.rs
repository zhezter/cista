use crate::{CoreResult, Vault};

impl Vault {
    pub fn to_json_bytes(&self) -> CoreResult<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn from_json_bytes(bytes: &[u8]) -> CoreResult<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
