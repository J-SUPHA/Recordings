use super::prompts;
use crate::AppError;
use std::process::Command;
use std::str::FromStr;
use std::collections::VecDeque;

// main splitter so that the LLM can handle the text that is coming in

struct EmbeddingMiddle {
    text: String,
    embedding: Option<Vec<f32>>,
    distance_to_next: Option<f32>,
}

impl EmbeddingMiddle {

    fn new_empty() -> Self {
        Self {
            text: String::new(),
            embedding: None,
            distance_to_next: None,
        }
    }
    fn new(text: String, embedding: Option<Vec<f32>>,distance_to_next: Option<f32>) -> Self {
        Self {
            text,
            embedding,
            distance_to_next,
        }
    }
    fn new_embedding(text: String, embedding: Option<Vec<f32>>) -> Self {
        Self {
            text,
            embedding,
            distance_to_next: None,
        }
    }
}

use std::fs::OpenOptions;
use std::io::Write;

fn write_error_to_file(error: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("error_log.txt")?;

    writeln!(file, "Error: {:?}", error)?;

    Ok(())
}


pub fn split_via_sentences(input: &str) -> Vec<String> {
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut total: Vec<String> = Vec::new();

    let mut temp = String::new();
    for c in input.chars() {
        temp.push(c);
        if c == '.' {
            if queue.len() == 3 {
                let _front = queue.pop_front();
                queue.push_back(temp.clone());
                temp.clear();
                let joined_string = queue.iter().map(|s| s.clone()).collect::<Vec<String>>().join(" ");
                total.push(joined_string);
            }else{
                queue.push_back(temp.clone());
                temp.clear();
                let joined_string = queue.iter().map(|s| s.clone()).collect::<Vec<String>>().join(" ");
                total.push(joined_string);
            }
        }
    }
    while !queue.is_empty() {
        let _front = queue.pop_front();
        let joined_string = queue.iter().map(|s| s.clone()).collect::<Vec<String>>().join(" ");
        total.push(joined_string);
    }
    println!("Here is my total sentence {:?}", total.len());
    return total;
    
}


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

fn parse_embedding(output: &str) -> Vec<f32> {
    output
        .trim_start_matches("embedding 0:")
        .split_whitespace()
        .filter_map(|s| f32::from_str(s).ok())
        .collect()
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

use reqwest::{Client, StatusCode};
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
    println!("This is the request body {:?}", request_body);
    while retry_attempt < retry_count {
        println!("Another call");
        let response_result = client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", groq_key.clone()))
            .json(&request_body)
            .timeout(timeout)
            .send()
            .await;

        match response_result {
            Ok(response) => {
                println!("The response is {:?}", response);
                match response.status() {
                    StatusCode::OK => {

                        let intermediate = response.text().await.map_err(|e| e.to_string())?;
                        let json: Value = serde_json::from_str(&intermediate).map_err(|e| e.to_string())?;
                        let content = json["choices"][0]["message"]["content"]
                            .as_str()
                            .ok_or_else(|| "Falied to extract content from JSON".to_string())?
                            .to_string();
                        println!("The error originated here");
                        return Ok(content);
                    }
                    StatusCode::TOO_MANY_REQUESTS => {
           
                        println!("Too many requests. Retrying...");
                        retry_attempt += 1;
                        if retry_attempt < retry_count {
                            tokio::time::sleep(Duration::from_secs(60)).await;
                            continue;
                        }else {
                            return Err("Failed to send request after 3 retries".to_string());
                        }
                    }
                    _=> {
                        println!("Something else is going on here");
                        let error_message = format!(
                            "Unexpected response status: {}",
                            response.status()
                        );
                        return Err(error_message);
                    }
                }
            }
            Err(error) => {
                eprintln!("line 203: There way no status when send to the Groq API for sime reason: {:?}", error);
                retry_attempt+=1;
                if retry_attempt < retry_count {
                    return Err(format!("Failed to send request after {} retries", retry_attempt));
                }else {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        }
    }
    return Err("Failed to send request after 3 retries".to_string());
}


pub async fn summarize_and_send(
    groq_key: String,
    total: &Vec<String>,
    action : bool
) -> Result<(), AppError> {

    let mut prompt: Vec<prompts::Message> = if action {
        prompts::ACTION.to_vec()
    }else {
        prompts::MINOR.to_vec()
    };
    
    let mut google_output = String::new();
    for items in total.clone() {
        prompt.push( 
            prompts::Message{
                role: "user".to_string(),
                content: items
            }
        );
        let request_body = serde_json::json!({
            "model": "Llama3-70b-8192",
            "messages": prompt.clone()
        });
        prompt.pop();
        let response = send_groq_api_request(groq_key.clone(), request_body);

        match response.await {
            Ok(response) => {
                google_output.push_str(&response);
            }
            Err(e) => {
                eprintln!("Line 249: Error occured while sending request to Groq API: {:?} the actual response was",e);
                return Err(AppError::Other(e));
            }
        }
    }
    println!("{:?} this is the google output ",google_output);
    let output = Command::new("python3")
        .arg("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/google_docs.py")
        .arg("--write")
        .arg(google_output)
        .output()
        .expect("Failed to execute command");
    println!("Status {}", output.status);
    println!("Output {}", String::from_utf8_lossy(&output.stdout));
    println!("Error {}", String::from_utf8_lossy(&output.stderr));
    return Ok(())

    // 1HFD4EzZqm_i_AUn3NcbI1Bz8rZNRpENqQuB4oNGmbKY this is the document ID

}


pub async fn summarize_raw(groq_key: String, text: String, action: bool) -> Result<(), AppError> {
    let mut prompt: Vec<prompts::Message> = if action {
        prompts::ACTION.to_vec()
    }else {
        prompts::MINOR.to_vec()
    };
    prompt.push(
        prompts::Message{
            role: "user".to_string(),
            content: text
        }
    );
    let request_body = serde_json::json!({
        "model": "Llama3-70b-8192",
        "messages": prompt.clone()
    });
    prompt.pop();
    println!("This is the request body {:?}", request_body);
    let response = send_groq_api_request(groq_key.clone(), request_body);

    match response.await {
        Ok(response) => {
            println!("This is my response {:?}", response);
            let output = Command::new("python3")
                .arg("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/google_docs.py")
                .arg("--write")
                .arg(response)
                .output()
                .expect("Failed to execute command");
            println!("Status: {}", output.status);
            println!("Output: {}", String::from_utf8_lossy(&output.stdout));
            println!("Error: {}", String::from_utf8_lossy(&output.stderr));
            return Ok(())
        }
        Err(e) => {
            eprintln!("Error occurred while trying to run google_doc.py: {:?}", e);
            return Err(AppError::Other(e));
        }
    }
}


pub async fn embeddings(text: &String) -> Result<Vec<f32>, AppError> {
    let command = format!(
        "/Users/j-supha/Desktop/Personal_AI/FFMPEG/llama.cpp/llama-embedding -m /Users/j-supha/Desktop/Personal_AI/FFMPEG/models/EMB/gguf/mxbai-embed-large-v1-f16.gguf --prompt {}",
        shell_escape::escape(text.into())
    );

    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("Failed to execute command");

    let output = String::from_utf8_lossy(&output.stdout);

    write_error_to_file(output.to_string().as_str()).expect("Failed to write to error file");


    let embeddings = parse_embedding(&output);
    Ok(embeddings)
}

fn percentile(mut numbers: Vec<f32>, percentile: f32) -> Option<f32> {

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let index = (numbers.len() as f32 * percentile).floor() as usize;
    numbers.get(index).cloned()
}   


pub async fn sem_tag_process(
    text: String
) -> Result<Vec<String>, AppError> {

    let mut return_vec: Vec<String> = Vec::new();

    let mut ninety: Vec<f32> = Vec::new();
    let my_vec = split_via_sentences(&text);
    println!("The length of the vector is {:?}", my_vec.len());
    let mut temp = EmbeddingMiddle::new_empty();
    let mut total: Vec<EmbeddingMiddle> = Vec::new();
    for i in 0..my_vec.len() -1 {
        if temp.text.is_empty() {
            let cur_emb = embeddings(&my_vec[i]).await.map_err(|e| AppError::Other(e.to_string()))?;
            let next_emb = embeddings(&my_vec[i+1]).await.map_err(|e| AppError::Other(e.to_string()))?;
            let cosine_similarity = cosine_similarity(&cur_emb, &next_emb);
            println!("The cosine similarity is {:?}", cosine_similarity);
            ninety.push(cosine_similarity.clone());
            total.push(EmbeddingMiddle::new(my_vec[i].clone(), Some(cur_emb),Some(cosine_similarity)));
            temp = EmbeddingMiddle::new_embedding(my_vec[i+1].clone(), Some(next_emb));
        }else {
            let next_emb = embeddings(&my_vec[i+1]).await.expect("Failed to get the model embeddings");
            let store = temp.embedding.clone();
            let cosine_similarity = cosine_similarity(&temp.embedding.unwrap(), &next_emb);
            println!("The cosine similarity is {:?}", cosine_similarity);
            ninety.push(cosine_similarity.clone());
            total.push(EmbeddingMiddle::new(my_vec[i].clone(),store ,Some(cosine_similarity)));
            temp = EmbeddingMiddle::new_embedding(my_vec[i+1].clone(), Some(next_emb));
        }
    }


    let thresh = percentile(ninety, 0.9).expect("Failed to get the 90th percentile");
    let mut str_buf = String::new();
    for items in total {
        if items.distance_to_next.unwrap() < thresh {
            return_vec.push(str_buf.clone());
            str_buf.clear();
        }
        str_buf.push_str(&items.text);
    }
    return Ok(return_vec);

}


pub async fn rag_tag_process(
    groq_key: String,
    text:String
) -> Result<Vec<String>, AppError> {
    let my_vec = split_into_chunks(&text,2000);

    let mut unfinished = String::new();
    let mut total: Vec<String> = Vec::new();
    let mut prompt = prompts::MAJOR.to_vec();
    for vectors in my_vec {
        let insert = format!("{}\n{}", unfinished, vectors);
        prompt.push(
            prompts::Message{
                role: "user".to_string(),
                content: insert.clone()
            }
        );
        let request_body = serde_json::json!({
            "model": "Llama3-70b-8192",
            "messages": prompt.clone()
        });
        prompt.pop();
        let cum_str = send_groq_api_request(groq_key.clone(), request_body);

        match cum_str.await {
            Ok(response) => {
                let (finished, unfinished_text) = parse_topics(&response);
                total.extend(finished);
                unfinished = unfinished_text;
            }
            Err(e) => {
                eprintln!("Line 398: Error occured while sending request to Groq API: {:?}", e);
                return Err(AppError::Other(e));
            }
        }
    }
    return Ok(total);
}


fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(a, b)| a * b).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    return dot_product / (norm_a * norm_b);
}

async fn chat(groq_key: String, input: Vec<(String, Option<Vec<f32>>)>) -> Result<(), AppError> {
    loop {
        println!("Ask a question based on the transcript or enter exit to leave");
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();
        match choice{
            "exit" => {
                break;
            }
            _ => {
                let my_choice = choice.to_string();
                let vector = embeddings(&my_choice);
                match vector.await {
                    Ok(vector) => {
                        println!("The first problem {:?}", input[0]);
                        let similarities: Vec<(f32, &String)> = input.iter()
                            .filter_map(|(text, embedding)| {
                                embedding.as_ref().map(|emb| {
                                    let similarity = cosine_similarity(&vector, emb);
                                    (similarity, text)
                                })
                            })
                            .collect();
                        println!("The similarities {:?}", similarities);
                        let mut valid_similarities: Vec<_> = similarities
                            .into_iter()
                            .filter(|(score, _)| !score.is_nan())
                            .collect();

                        valid_similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                        let mut prompt = prompts::CHAT.to_vec();

                        println!("The valid similarities {:?}", valid_similarities);
                        println!("Valid similarities {:?}", valid_similarities.len());
                        println!("The first problem {:?}", valid_similarities[0]);
                        prompt.push(
                            prompts::Message{
                                role: "user".to_string(),
                                content: format!("{}\n{}\n{}\n{}", valid_similarities[0].1, valid_similarities[1].1, valid_similarities[2].1, my_choice)
                            }
                        );
                        let request_body = serde_json::json!({
                            "model": "Llama3-70b-8192",
                            "messages": prompt.clone()
                        });
                        let response = send_groq_api_request(groq_key.clone(), request_body);

                        match response.await {
                            Ok(response) => {
                                println!("Here is the reponse that was given {}", response);
                            }
                            Err(e) => {
                                eprintln!("Line 467: Error occured while sending request to Groq API: {:?}", e);
                                return Err(AppError::Other(e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error occured fetching the vector: {:?}", e);
                        return Err(AppError::Other(e.to_string()));
                    }
                }
            }
        }
    }
    Ok(())
}


pub async fn chat_or_summarize(input: Vec<(String,Option<Vec<f32>>)>, groq_key: String) -> Result<(), AppError> {

    println!("Here is my action plan");

    loop {
        println!("Enter 1 for a sumarization, 2 for an action plan, 3 to chat with the transcript, 4 to exit");
        let mut choice = String::new();
        std::io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();
        let mut bool = false;
        match choice {
            "1" => {
                summarize_and_send(groq_key.clone(), &input.iter().map(|(x,_)| x.clone()).collect(), false).await?;
            }
            "2" => {
                summarize_and_send(groq_key.clone(), &input.iter().map(|(x,_)| x.clone()).collect(), true).await?;
            }
            "3" => {
                chat(groq_key.clone(), input.clone()).await?;
            }
            "4" => {
                bool = true;
            }
            _ => {
                println!("Invalid choice");
            }
        }
        if bool {
            break;
        }

    }
    Ok(())
    
}