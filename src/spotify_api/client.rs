use crate::config;
use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::Method;
use std::time::{SystemTime, UNIX_EPOCH};

use super::oauth::Token;

pub const API_BASE_URL: &str = "https://api.spotify.com/v1";

#[derive(Debug, Clone)]
pub struct APIClient {
    pub client_id: String,
    pub token: Option<Token>,
    pub(crate) http: Client,
}

#[derive(Debug)]
pub enum ApiError {
    NotAuthenticated(String),
    Request(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotAuthenticated(msg) => write!(f, "not authenticated: {}", msg),
            ApiError::Request(msg) => write!(f, "request failed: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

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

    fn bearer(&self) -> Result<String, ApiError> {
        if let Some(token) = &self.token {
            Ok(format!("Bearer {}", token.access_token))
        } else {
            Err(ApiError::NotAuthenticated("missing token".into()))
        }
    }

    pub fn api_request(&mut self, method: Method, endpoint: &str) -> Result<String, ApiError> {
        if !self.is_authenticated() {
            self.refresh_access_token()?;
        }
        let url = format!("{}{}", API_BASE_URL, endpoint);
        let auth = self.bearer()?;
        let resp = self
            .http
            .request(method.clone(), &url)
            .header("Authorization", auth)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            self.refresh_access_token()?;
            return self.api_request(method, endpoint);
        }

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(ApiError::Request(text));
        }

        let text = resp.text().unwrap_or_default();
        Ok(text)
    }

    pub fn api_request_with_body(
        &mut self,
        method: Method,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<(), ApiError> {
        if !self.is_authenticated() {
            self.refresh_access_token()?;
        }
        let url = format!("{}{}", API_BASE_URL, endpoint);
        let auth = self.bearer()?;
        let resp = self
            .http
            .request(method.clone(), &url)
            .header("Authorization", auth)
            .json(&body)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if resp.status().as_u16() == 401 {
            self.refresh_access_token()?;
            return self.api_request_with_body(method, endpoint, body);
        }

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(ApiError::Request(text));
        }

        Ok(())
    }

    pub fn refresh_access_token(&mut self) -> Result<(), ApiError> {
        let refresh = match &self.token {
            Some(t) if !t.refresh_token.is_empty() => t.refresh_token.clone(),
            _ => return Err(ApiError::NotAuthenticated("no refresh token".into())),
        };

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("client_id", self.client_id.as_str()),
        ];

        let resp = self
            .http
            .post(super::oauth::TOKEN_URL)
            .form(&params)
            .send()
            .map_err(|e| ApiError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(ApiError::NotAuthenticated(text));
        }

        let mut token: Token = resp.json().map_err(|e| ApiError::Request(e.to_string()))?;
        if let Some(existing) = &self.token {
            if token.refresh_token.is_empty() {
                token.refresh_token = existing.refresh_token.clone();
            }
        }
        token.expires_at = Token::calc_expiry(token.expires_in);
        self.token = Some(token.clone());
        config::save_token(&token).map_err(|e| ApiError::Request(e.to_string()))?;
        Ok(())
    }

    pub fn load_token_from_disk(&mut self) -> Result<()> {
        if let Ok(token) = config::load_token() {
            self.set_token(token);
        }
        Ok(())
    }
}
