use anyhow::{bail, Context};
use reqwest::Method;
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct ApiClient {
    http: reqwest::Client,
    api_url: String,
    session_token: Option<String>,
    owner_pubkey: Option<String>,
}

impl ApiClient {
    pub(crate) fn new(
        api_url: String,
        session_token: Option<String>,
        owner_pubkey: Option<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_url,
            session_token,
            owner_pubkey,
        }
    }

    pub(crate) async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        auth: bool,
        owner_header: bool,
    ) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.api_url.trim_end_matches('/'), path);
        let mut request = self.http.request(method, url);
        if auth {
            let token = self
                .session_token
                .as_deref()
                .context("not authenticated; run shipyard auth login")?;
            request = request.bearer_auth(token);
        }
        if owner_header {
            if let Some(owner_pubkey) = &self.owner_pubkey {
                request = request.header("x-shipyard-owner-pubkey", owner_pubkey);
            }
        }
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.context("request failed")?;
        let status = response.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::json!({ "status": "ok" }));
        }

        let value = response
            .json::<Value>()
            .await
            .context("invalid JSON response")?;
        if !status.is_success() {
            bail!("{}", value);
        }
        Ok(value)
    }

    pub(crate) fn required_owner(&self) -> anyhow::Result<&str> {
        self.owner_pubkey
            .as_deref()
            .context("owner pubkey required; use --owner-pubkey or shipyard accounts use")
    }
}
