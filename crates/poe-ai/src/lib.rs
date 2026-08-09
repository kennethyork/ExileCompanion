use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    think: bool,
    keep_alive: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
}

#[derive(Debug, Clone)]
pub struct OllamaClient {
    endpoint: String,
    client: reqwest::blocking::Client,
}

impl OllamaClient {
    pub fn new(endpoint: &str) -> Result<Self> {
        let endpoint = endpoint.trim();
        let parsed = reqwest::Url::parse(endpoint).context("invalid Ollama endpoint")?;
        let is_loopback = matches!(
            parsed.host_str(),
            Some("127.0.0.1" | "localhost" | "::1" | "[::1]")
        );
        let has_unexpected_url_parts = !parsed.username().is_empty()
            || parsed.password().is_some()
            || !matches!(parsed.path(), "" | "/")
            || parsed.query().is_some()
            || parsed.fragment().is_some();
        if parsed.scheme() != "http" || !is_loopback || has_unexpected_url_parts {
            bail!("only local Ollama endpoints are allowed");
        }
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self { endpoint, client })
    }

    pub fn models(&self) -> Result<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.endpoint))
            .send()
            .context("could not connect to Ollama")?
            .error_for_status()?;
        Ok(response
            .json::<TagsResponse>()?
            .models
            .into_iter()
            .map(|model| model.name)
            .collect())
    }

    pub fn chat(&self, model: &str, messages: &[ChatMessage]) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/api/chat", self.endpoint))
            .json(&ChatRequest {
                model,
                messages,
                stream: false,
                think: false,
                keep_alive: "10m",
            })
            .send()
            .context("could not connect to Ollama; make sure `ollama serve` is running")?
            .error_for_status()
            .context("Ollama rejected the chat request")?;
        let content = response.json::<ChatResponse>()?.message.content;
        if content.trim().is_empty() {
            bail!("Ollama returned an empty answer");
        }
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_remote_endpoints() {
        assert!(OllamaClient::new("https://example.com").is_err());
        assert!(OllamaClient::new("http://localhost.attacker.example").is_err());
        assert!(OllamaClient::new("http://localhost@attacker.example").is_err());
        assert!(OllamaClient::new("http://127.0.0.1.attacker.example").is_err());
    }

    #[test]
    fn accepts_loopback_endpoints() {
        assert!(OllamaClient::new("http://127.0.0.1:11434").is_ok());
        assert!(OllamaClient::new("http://localhost:11434/").is_ok());
        assert!(OllamaClient::new("http://[::1]:11434").is_ok());
    }
}
