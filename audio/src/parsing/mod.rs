// use portaudio::Input;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs;
use std::env;
use crate::error::AppError;
mod helper;
use helper::{split_into_chunks, parse_topics, send_groq_api_request, summarize_raw, summarize_and_send};
mod prompts;
use prompts::Message;
mod db;
use db::Database;
use std::io::{self, Write};



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
    groq_key: String,
}


fn parse_embedding(output: &str) -> Vec<f32> {
    output
        .trim_start_matches("embedding 0:")
        .split_whitespace()
        .filter_map(|s| f32::from_str(s).ok())
        .collect()
}

impl Sst {
    pub fn new(audio_file: String, model_path: String) -> Self {
        Self {
            audio_file,
            model_path,
            groq_key: env::var("GROQ_API_KEY").expect("GROQ key not found within envrionment variables."),
        }
    }

    // Method to extract text from audio file using the whisper.cpp binary
    fn extract_text_from_audio(&self) -> Result<String, AppError> {

        println!("Extracting text from audio file...");

        let output_txt = format!("{}.txt", self.audio_file);

        println!("Extracting text from audio file...");

        let command = format!("/Users/j-supha/Desktop/Personal_AI/FFMPEG/whisper.cpp/main --model {} --output-txt {} {}", self.model_path, output_txt, self.audio_file);

        println!("This is my model path: {}", self.model_path);
        println!("This is my output path: {}", output_txt);
        println!("This is my audio file: {}", self.audio_file);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()?;

        println!("Passed 1");

        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        println!("Passed 2");

        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
        println!("Passed 3");

        // Read the output from the text file
        let text = fs::read_to_string(output_txt)?;

        println!("Passed 4");

        Ok(text)
    }


    // async fn semantic_rag(&mut self text: String) -> Result<(), AppError> {

    // }

    // Method to send extracted text to API and handle response
    async fn rag_tag(&mut self, text: String) -> Result<(), AppError> { // must correct the input structure
        // reqwest client to send requests

        // Split the text into chunks of 2000 characters -
        let my_vec = split_into_chunks(&text, 2000);

        println!("writing {:?}", my_vec.len());

        let mut f = String::new(); // Will marke the unfinished tags that occur within the text. So will take into account any open <topic> tags that are still in place 
        let mut total: Vec<String> = Vec::new(); // Will store the actual chunked text - probably the most important vector in the whole pipeline 
        let mut init_vec = prompts::MAJOR.to_vec();

        for vectors in my_vec {

            let insert = format!("{} {}", f,vectors);
            init_vec.push(Message {
                role: "user".to_string(),
                content: insert.clone(),
            });

            let request_body = serde_json::json!({
                "model": "Llama3-70b-8192",
                "messages": init_vec
            });
            init_vec.pop();

            let cum_str = send_groq_api_request(self.groq_key.clone(), request_body);

            match cum_str.await {
                Ok(response_text) => {
                    let (finished, unfinished) = parse_topics(&response_text);
                    f = unfinished;

                    for i in &finished {
                        total.push(i.clone());
                    }
                }
                Err(error_message) => {
                    eprintln!("Error: {}", error_message);
                }
            }
        }

        // We need to initialize a database if it has not been initialized already then we need to insert the text into the database using the
        // text primary key with the the file-system in place. so that each text is unique ffo something along tjhe lines of forge_meets_1_snippet_1
        // while insert is going one we need to call the embeddings model and embedd the actual text that is going into the model

        let db = Database::new("audio_text.db")?;
        db.init().await?;

        // go back and search for the name of the transcript file 
        // name of the audio file is found
        let sec_key = format!("RAGTAG_{}", &self.audio_file);
        let answer = db.check_if_audio_exists(&sec_key).await; // check if the audio file exists in the database

        match answer {
            Ok(true) => {
                println!("Audio file already exists in the database. will retrieve the data.");
                // we can store the data - but then append and then use the data directly
                let answer = db.get(&sec_key).await?;
            }
            Ok(false) => {
                println!("Audio file does not exist in the database.");
                // ./llama-embedding -m ../models/EMB/gguf/mxbai-embed-large-v1-f16.gguf --prompt "Your text here"

                for (index,items) in total.clone().into_iter().enumerate() {
                    let output = Command::new("sh")
                        .arg("-c")
                        .arg(format!("/Users/j-supha/Desktop/Personal_AI/FFMPEG/llama.cpp/llama-embedding -m /Users/j-supha/Desktop/Personal_AI/FFMPEG/models/EMB/gguf/mxbai-embed-large-v1-f16.gguf --prompt '{}'", items))
                        .output()
                        .expect("Failed to execute command 1");
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    let embedding = parse_embedding(&output_str);
                    let primary_key = format!("{}_{}", index, &self.audio_file);
                    let secondary_key = format!("RAGTAG_{}", &self.audio_file);
                    db.insert(&primary_key, &secondary_key, &items, Some(&embedding)).await?;

                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
        // so now llm contains a string embedding of the each of the conversations. they have been summarized into segments but nothing has been done
        // the only thing that has been done is that they have been separated into topics and summarized
        // in the current scope of work at least for this cycle we are going to implement the following steps
        // 1 . naive - write the summarized output straight into a google doc - this has already been done with _output
        // 2. add another parsing step to the raw chunk ask the llm to extract any and all action items and then write those down into a google doc
        // 3. add a databse integration with rag and allow a user to chat directly with the meeting notes
        // 4  add optionality so that the user can choose exactly what they want to do with all this information

        // within total before all of this we need to 

        Ok(())
    }

    // Combined method to process audio file and handle API interaction
    pub async fn process_audio_file(&mut self) -> Result<(), AppError> {
        println!("Processing audio file...");
        let text = self.extract_text_from_audio()?;
        if text.len() < 8000 {
            return summarize_raw(self.groq_key.clone(), text.clone(), true).await;
        }else{
            loop {
                println!("You can either use RAGTAG or you can use Semantic RAG to use RAGTAG type in R, to use Semantic RAG type in S, to exit type in E");
                io::stdout().flush().unwrap();
                let mut command = String::new();
                io::stdin().read_line(&mut command).unwrap();
                let command = command.trim();
                if command == "E" {
                    break;
                }
                if command == "R" {
                    self.rag_tag(text.clone()).await?;
                }
                if command == "S" {
                    println!("Not yet implemented");
                    // self.semantic_rag(text).await?;
                }
            }
        }
        return Ok(());
    }
}