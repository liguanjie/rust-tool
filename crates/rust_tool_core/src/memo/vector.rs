use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    store: Option<bool>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String, // "json_object"
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

pub struct LlmClient {
    base_url: String,
    api_key: Option<String>,
    chat_model: String,
    embedding_model: String,
    reasoning_effort: Option<String>,
    disable_response_storage: bool,
    client: reqwest::Client,
}

impl LlmClient {
    pub fn new(
        base_url: &str,
        api_key: Option<&str>,
        chat_model: &str,
        embedding_model: &str,
        reasoning_effort: Option<&str>,
        disable_response_storage: bool,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(|k| k.to_string()),
            chat_model: chat_model.to_string(),
            embedding_model: embedding_model.to_string(),
            reasoning_effort: reasoning_effort
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            disable_response_storage,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60)) // Large timeout for local LLM inference
                .build()
                .unwrap_or_default(),
        }
    }

    /// Retrieve embedding vector for a piece of text using OpenAI standard /embeddings API.
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/embeddings", self.base_url);
        let req = EmbeddingRequest {
            model: self.embedding_model.clone(),
            input: text.to_string(),
        };

        let mut req_builder = self.client.post(&url).json(&req);
        if let Some(ref key) = self.api_key {
            if !key.trim().is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
            }
        }

        let res = req_builder
            .send()
            .await
            .map_err(|e| format!("LLM embedding request failed: {:?}", e))?;

        if !res.status().is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("LLM returned error ({}): {}", url, err_body));
        }

        let resp: EmbeddingResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse embedding response: {:?}", e))?;

        resp.data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| "Empty embedding data returned from LLM".to_string())
    }

    /// Send a chat prompt using OpenAI standard /chat/completions API.
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        json_mode: bool,
    ) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url);

        let response_format = if json_mode {
            Some(ResponseFormat {
                format_type: "json_object".to_string(),
            })
        } else {
            None
        };

        let req = ChatRequest {
            model: self.chat_model.clone(),
            messages,
            response_format,
            reasoning_effort: self.reasoning_effort.clone(),
            store: self.disable_response_storage.then_some(false),
            temperature: 0.1, // Low temperature for deterministic/factual results
        };

        let mut req_builder = self.client.post(&url).json(&req);
        if let Some(ref key) = self.api_key {
            if !key.trim().is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
            }
        }

        let res = req_builder
            .send()
            .await
            .map_err(|e| format!("LLM chat request failed: {:?}", e))?;

        if !res.status().is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(format!("LLM chat returned error: {}", err_body));
        }

        let resp: ChatResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse chat response: {:?}", e))?;

        resp.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| "Empty chat response choice returned from LLM".to_string())
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-5);
    }
}
