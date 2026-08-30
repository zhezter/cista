use crate::SecretString;
use crate::{CoreError, CoreResult};
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    id: Uuid,
    name: String,
    username: Option<String>,
    password: Secret<SecretString>,
    url: Option<String>,
    notes: Option<Secret<SecretString>>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl Entry {
    pub fn new(
        name: String,
        username: Option<String>,
        password: Secret<SecretString>,
        url: Option<String>,
        notes: Option<String>,
    ) -> CoreResult<Self> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(CoreError::EmptyName);
        }
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: Uuid::new_v4(),
            name: trimmed_name.to_string(),
            username,
            password,
            url,
            notes: notes.map(|n| Secret::new(SecretString::from(n))),
            created_at: now,
            updated_at: now,
        })
    }
}

impl Entry {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }

    pub fn updated_at(&self) -> OffsetDateTime {
        self.updated_at
    }
}

impl Entry {
    pub fn password(&self) -> &Secret<SecretString> {
        &self.password
    }

    pub fn notes(&self) -> Option<&Secret<SecretString>> {
        self.notes.as_ref()
    }
}

impl Entry {
    pub fn rename(&mut self, new_name: String) -> CoreResult<()> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::EmptyName);
        }
        self.name = trimmed.to_string();
        self.touch();
        Ok(())
    }

    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
        self.touch();
    }

    pub fn set_password(&mut self, password: Secret<SecretString>) {
        self.password = password;
        self.touch();
    }

    pub fn set_url(&mut self, url: Option<String>) {
        self.url = url;
        self.touch();
    }

    pub fn set_notes(&mut self, notes: Option<String>) {
        self.notes = notes.map(|n| Secret::new(SecretString::from(n)));
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = OffsetDateTime::now_utc();
    }
}
