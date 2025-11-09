use crate::error::AppError;
use crate::services::prompts;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20241022";
const MAX_TOKENS: u32 = 4096;

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[allow(dead_code)]
    id: String,
    content: Vec<ContentBlock>,
    usage: Usage,
    model: String,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug)]
pub struct AiResponse {
    pub text: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub model_used: String,
}

/// Generate AI-suggested changes based on a comment
pub async fn generate_plan_changes(
    api_key: &str,
    plan_content: &str,
    comment_text: &str,
    line_start: i32,
    line_end: i32,
) -> Result<AiResponse, AppError> {
    // Validate API key
    if api_key.trim().is_empty() {
        return Err(AppError::BadRequest("API key is required".to_string()));
    }

    // Extract the relevant lines from the plan
    let lines: Vec<&str> = plan_content.lines().collect();
    let start_idx = (line_start - 1).max(0) as usize;
    let end_idx = (line_end as usize).min(lines.len());
    let relevant_lines = lines[start_idx..end_idx].join("\n");

    // Create the prompt using the prompts module
    let prompt =
        prompts::generate_plan_review_prompt(&relevant_lines, line_start, line_end, comment_text);

    // Build the request
    let request = AnthropicRequest {
        model: DEFAULT_MODEL.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: prompt,
        }],
    };

    // Make the API call
    let client = Client::new();
    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to call Anthropic API: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(AppError::Internal(format!(
            "Anthropic API error ({}): {}",
            status, error_text
        )));
    }

    let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
        AppError::Internal(format!("Failed to parse Anthropic API response: {}", e))
    })?;

    // Extract the text from the first content block
    let text = anthropic_response
        .content
        .first()
        .map(|block| block.text.clone())
        .unwrap_or_default();

    Ok(AiResponse {
        text,
        prompt_tokens: anthropic_response.usage.input_tokens as i32,
        completion_tokens: anthropic_response.usage.output_tokens as i32,
        model_used: anthropic_response.model,
    })
}

/// Apply AI-suggested changes to the plan content
pub fn apply_changes_to_plan(
    original_content: &str,
    suggested_changes: &str,
    line_start: i32,
    line_end: i32,
) -> String {
    let mut lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();
    let start_idx = (line_start - 1).max(0) as usize;
    let end_idx = (line_end as usize).min(lines.len());

    // Replace the specified line range with the AI-suggested changes
    let new_lines: Vec<String> = suggested_changes.lines().map(|s| s.to_string()).collect();

    // Remove old lines and insert new ones
    lines.splice(start_idx..end_idx, new_lines);

    lines.join("\n")
}
