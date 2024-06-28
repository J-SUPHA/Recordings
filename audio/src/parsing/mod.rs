// use portaudio::Input;

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs;
use std::env;
use crate::error::AppError;
mod helper;
use helper::{summarize_raw, rag_tag_process, chat_or_summarize, embeddings, sem_tag_process};
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



    async fn chunking_tag(&mut self, text: String, rag_tag: bool) -> Result<(), AppError> { // must correct the input structure
        // reqwest client to send requests
        let db = Database::new("audio_text.db")?;
        db.init().await?;

        // go back and search for the name of the transcript file 
        // name of the audio file is found

        let sec_key = if rag_tag {
            format!("RAGTAG_{}", &self.audio_file)
        } else {
            format!("SEMTAG_{}", &self.audio_file)
        };

        let answer = db.check_if_audio_exists(&sec_key).await; // check if the audio file exists in the database

        match answer {
            Ok(true) => {
                println!("Audio file already exists in the database. will retrieve the data.");

                let answer = db.get(&sec_key).await?;
                chat_or_summarize(answer, self.groq_key.clone()).await?;
            }
            Ok(false) => {
                println!("Audio file does not exist in the database.");

                if rag_tag {
                    let total = rag_tag_process(self.groq_key.clone(), text.clone()).await?;

                    let mut combined_data: Vec<(String, Option<Vec<f32>>)> = Vec::new();

                    for (index,items) in total.into_iter().enumerate() {
                        let embedding = embeddings(&items).await?;
                        let primary_key = format!("{}_{}", index, &self.audio_file);
                        let secondary_key  = format!("RAGTAG_{}", &self.audio_file);
                        db.insert(&primary_key, &secondary_key, &items, Some(&embedding)).await?;
                        combined_data.push((items, Some(embedding)));
                    }
                    chat_or_summarize(combined_data, self.groq_key.clone()).await?;
                } else {

                    let total = sem_tag_process(text.clone()).await?;

                    let mut combined_data: Vec<(String, Option<Vec<f32>>)> = Vec::new();

                    for (index,items) in total.into_iter().enumerate() {
                        let embedding = embeddings(&items).await?;
                        let primary_key = format!("{}_{}", index, &self.audio_file);
                        let secondary_key  = format!("RAGTAG_{}", &self.audio_file);
                        db.insert(&primary_key, &secondary_key, &items, Some(&embedding)).await?;
                        combined_data.push((items, Some(embedding)));
                    }
                    chat_or_summarize(combined_data, self.groq_key.clone()).await?;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }


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
                    self.chunking_tag(text.clone(), true).await?;
                }
                if command == "S" {
                    self.chunking_tag(text.clone(), false).await?;
                }
            }
        }
        return Ok(());
    }
}