use serde::{Deserialize, Serialize};
use std::process::Command;
use reqwest::Client;
use std::fs;
use crate::error::AppError;
mod helper;
use helper::{split_into_chunks, parse_topics};
mod prompts;



#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}


#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    model: String,
    created_at: String,
    message: Message,
    done: bool,
}

pub struct Sst {
    audio_file: String,
    model_path: String,
}

impl Sst {
    pub fn new(audio_file: String, model_path: String) -> Self {
        Self {
            audio_file,
            model_path,
        }
    }

    fn extract_text_from_audio(&self) -> Result<String, AppError> {
        println!("B4");
        let output_txt = format!("{}.txt", self.audio_file);
        println!("1");
        let command = format!("/Users/j-supha/FFMPEG/whisper.cpp/main --model {} --output-txt {} {}", self.model_path, output_txt, self.audio_file);
        println!("2");
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()?;
        println!("3");
        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        println!("4");
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        println!("5");
        // Read the output from the text file
        let text = fs::read_to_string(output_txt)?;
        println!("6 {:?}", text);
        Ok(text)
    }

    // Method to send extracted text to API and handle response
    async fn send_text_to_api(&mut self, text: String) -> Result<(), AppError> {
        println!("Sending text to API...");
        let client = Client::new();
        let my_vec = split_into_chunks(&text, 2000);
        let mut f = String::new();
        let mut total: Vec<String> = Vec::new();
        for vectors in my_vec {
            println!("Sending text to API for topic parsing");
            
            let insert = format!("{} {}", f,vectors);
            let wild = format!("NEW!: {} {}\n\n\n", f,vectors);
            fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/whatIsHappenning.txt", wild).expect("unable to write file");
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": []
            });
            let mut res = client.post("http://localhost:11434/api/chat")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| AppError::Other(e.to_string()))?;
            if res.status().is_success() {
                let mut cum_str = String::new();
                while let Some(chunk) = res.chunk().await.map_err(|e| AppError::Other(e.to_string()))? {
                    let api_response: ApiResponse = serde_json::from_slice(&chunk)?;
                    cum_str.push_str(&api_response.message.content);
                }
                let (finished, unfinished) = parse_topics(&cum_str);
                f = unfinished;
                for i in &finished {
                    total.push(i.clone());
                }
            }else{
                eprintln!("Failed to send request: {}", res.status());
            }
        }
        println!("finished tagging the text into the topic chunks");
        let mut llm = String::new();
        for items in total{
            println!("Passing chunk to be summzarized by LLM");
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": []
            });
            let mut res = client.post("http://localhost:11434/api/chat")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| AppError::Other(e.to_string()))?;
            if res.status().is_success() {
                let mut cum_str = String::new();
                while let Some(chunk) = res.chunk().await.map_err(|e| AppError::Other(e.to_string()))? {
                    let api_response: ApiResponse = serde_json::from_slice(&chunk)?;
                    cum_str.push_str(&api_response.message.content);
                }
                // Here is where I need to write ./google_docs.txt with the cum_str to.
                fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/google_docs.txt", &cum_str).expect("unable to write file");
                llm = format!("{:?}\n\n{:?}",llm, cum_str);

            }else{
                eprintln!("Failed to send request: {}", res.status());
            }

        }
        println!("This is the culmination of some hard work");
            let _output = Command::new("python3")
                .arg("google_docs.py")  // Path to the Python script
                .arg("--write")
                .arg(llm)               // Argument to pass to the Python script
                .output()                   // Executes the command as a child process
                .expect("Failed to execute command");
            // 1HFD4EzZqm_i_AUn3NcbI1Bz8rZNRpENqQuB4oNGmbKY this is the document ID

        Ok(())
    }

    // Combined method to process audio file and handle API interaction
    pub async fn process_audio_file(&mut self) -> Result<(), AppError> {
        println!("Processing audio file...");
        let text = self.extract_text_from_audio()?;
        println!("Extracted text from audio...");
        self.send_text_to_api(text).await?;
        Ok(())
    }
}