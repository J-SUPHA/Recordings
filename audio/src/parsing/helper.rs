

// main splitter so that the LLM can handle the text that is coming in
pub fn split_into_chunks(input: &str, chunk_size: usize) -> Vec<String> {
    println!("Splitting the text appropriately...");
    let chunks: Vec<String> = input
        .chars() // Convert the string into an iterator of characters
        .collect::<Vec<char>>() // Collect characters into a vector
        .chunks(chunk_size) // Split vector into chunks
        .map(|chunk| chunk.iter().collect()) // Convert each chunk back into a String
        .collect(); // Collect all chunks into a Vector
    return chunks;
}

// Topics parser to check how the llm handles topic parsing
pub fn parse_topics(response: &str) -> (Vec<String>, String) {
    let start_tag = "<topic>";
    let end_tag = "</topic>";
    let mut finished = Vec::new();
    let mut temp_buf = String::new();
    let mut flag = false;
    let mut i = 0;

    while i < response.len() {
        // Check for the start of a tag
        if response.as_bytes()[i] == b'<' {
            // Check if it's an end tag
            if i + 1 < response.len() && response.as_bytes()[i + 1] == b'/' {
                if response[i..].starts_with(end_tag) {
                    // If currently capturing, push to finished and reset
                    if flag {
                        finished.push(temp_buf.clone());
                        temp_buf.clear();
                        flag = false;
                    }
                    i += end_tag.len();
                    continue;
                }
            } else {
                // It's a start tag
                if response[i..].starts_with(start_tag) {
                    flag = true;
                    i += start_tag.len();
                    continue;
                }
            }
        }

        // If we are between tags, add to temp_buf
        if flag {
            temp_buf.push(response.as_bytes()[i] as char);
        }
        i += 1;
    }

    // Any data left after the last tag is considered unfinished
    let unfinished = temp_buf;

    (finished, unfinished)
}

use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub async fn send_groq_api_request(
    groq_key: String,
    request_body: Value,
) -> Result<String, String> {
    let client = Client::new();
    let timeout = Duration::from_secs(30);
    let retry_count = 3;
    let mut retry_attempt = 0;

    while retry_attempt < retry_count {
        let response_result = client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", groq_key.clone()))
            .json(&request_body)
            .timeout(timeout)
            .send()
            .await;

        match response_result {
            Ok(response) => {
                if response.status().is_success() {
                    let response_text = response.text().await.map_err(|e| e.to_string())?;
                    return Ok(response_text);
                } else {
                    let error_message = format!(
                        "Unexpected response status: {}",
                        response.status()
                    );
                    return Err(error_message);
                }
            }
            Err(error) => {
                eprintln!(
                    "Error occurred while sending request to Groq API: {:?}",
                    error
                );
                retry_attempt += 1;
                if retry_attempt == retry_count {
                    return Err(format!("Failed to send request after {} retries", retry_count));
                } else {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    Err("Unexpected execution flow".to_string())
}
