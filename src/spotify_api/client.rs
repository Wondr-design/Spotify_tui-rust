use anyhow::Result;
use reqwest::blocking::Client;
use std::time::{SystemTime, UNIX_EPOCH};

use super::oauth::Token;

#[derive(Debug, Clone)]
pub struct APIClient {
    pub client_id: String,
    pub token: Option<Token>,
    pub(crate) http: Client,
}

impl APIClient {
    pub fn new(client_id: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self {
            client_id,
            token: None,
            http,
        })
    }

    pub fn is_authenticated(&self) -> bool {
        if let Some(token) = &self.token {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            return now.as_secs() < token.expires_at.saturating_sub(10);
        }
        false
    }

    pub fn set_token(&mut self, token: Token) {
        self.token = Some(token);
    }
}
