use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Response, Server};
use url::Url;

use super::client::APIClient;
use crate::config;

pub const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
pub const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
pub const SCOPES: &str = "user-read-playback-state user-modify-playback-state user-read-currently-playing playlist-read-private user-library-read";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Token {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub expires_at: u64,
}

impl Token {
    pub fn calc_expiry(expires_in: u64) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        now.as_secs().saturating_add(expires_in)
    }
}

pub fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let hash = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hash);
    (verifier, challenge)
}

pub fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

impl APIClient {
    pub fn get_auth_url(&self, state: &str, challenge: &str) -> Result<String> {
        let mut url = Url::parse(AUTH_URL)?;
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", SCOPES)
            .append_pair("state", state)
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", challenge);
        Ok(url.to_string())
    }

    pub fn exchange_code(&mut self, code: &str, verifier: &str) -> Result<()> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &self.client_id),
            ("code_verifier", verifier),
        ];

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .map_err(|e| anyhow::anyhow!("failed to exchange code: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .context("failed to read token exchange response body")?;
        if !status.is_success() {
            anyhow::bail!("token exchange failed ({}): {}", status, body.trim());
        }

        let mut token: Token = serde_json::from_str(&body)
            .with_context(|| format!("invalid token response: {}", body.trim()))?;
        if token.access_token.is_empty() {
            anyhow::bail!(
                "token exchange returned empty access token: {}",
                body.trim()
            );
        }

        token.expires_at = Token::calc_expiry(token.expires_in);
        self.token = Some(token.clone());
        config::save_token(&token)?;
        Ok(())
    }

    pub fn start_auth_server(&mut self, state: &str, verifier: &str) -> Result<()> {
        let server = Server::http("0.0.0.0:8888")
            .map_err(|e| anyhow::anyhow!("failed to bind auth server: {}", e))?;
        let (tx, rx) = mpsc::channel::<Result<String>>();

        let state = state.to_string();
        thread::spawn(move || {
            if let Some(request) = server.incoming_requests().next() {
                let req_url: &str = request.url();
                let parsed = Url::parse(&format!("http://localhost{}", req_url));
                let code = match parsed {
                    Ok(url) => {
                        let mut code = None;
                        let mut got_state = None;
                        for (k, v) in url.query_pairs() {
                            if k == "code" {
                                code = Some(v.to_string());
                            }
                            if k == "state" {
                                got_state = Some(v.to_string());
                            }
                        }
                        if got_state.as_deref() != Some(&state) {
                            tx.send(Err(anyhow::anyhow!("state mismatch"))).ok();
                            return;
                        }
                        code
                    }
                    Err(e) => {
                        tx.send(Err(anyhow::anyhow!(e))).ok();
                        return;
                    }
                };

                let response = Response::from_string(
                    "<html><body><h1>SUCCESS!</h1><p>You can close this window.</p></body></html>",
                )
                .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
                let _ = request.respond(response);

                if let Some(code) = code {
                    tx.send(Ok(code)).ok();
                } else {
                    tx.send(Err(anyhow::anyhow!("missing code"))).ok();
                }
            }
        });

        let code = rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|e| anyhow::anyhow!("auth timeout: {}", e))??;
        self.exchange_code(&code, verifier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_is_url_safe() {
        let (verifier, challenge) = generate_pkce();
        assert!(!verifier.is_empty());
        assert!(!challenge.is_empty());
        assert!(!verifier.contains('='));
        assert!(!challenge.contains('='));
    }
}
