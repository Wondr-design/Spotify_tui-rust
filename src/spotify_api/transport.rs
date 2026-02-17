use reqwest::Method;

use super::client::APIClient;
use super::error::ApiError;

pub const API_BASE_URL: &str = "https://api.spotify.com/v1";

impl APIClient {
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
}
