use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::env;
use crate::error::AppError;
mod helper;
use helper::{split_into_chunks, parse_topics, send_groq_api_request};
mod prompts;
use prompts::Message;



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

    // Method to send extracted text to API and handle response
    async fn rag_tag(&mut self, text: String) -> Result<(), AppError> {
        // reqwest client to send requests

        println!("My RAG TAG");
        // Split the text into chunks of 2000 characters -
        let my_vec = split_into_chunks(&text, 2000);
        println!("writing {:?}", my_vec.len());
        fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/test.txt", &my_vec.join("\n\n")).expect("unable to write file");
        println!("Writing to a specific file ");
        let mut f = String::new(); // Will marke the unfinished tags that occur within the text. So will take into account any open <topic> tags that are still in place 
        let mut total: Vec<String> = Vec::new(); // Will store the actual chunked text
        let mut init_vec = prompts::MAJOR.to_vec();
        for vectors in my_vec {
            println!("Sending vectors to the correct spots");
            let insert = format!("{} {}", f,vectors);
            init_vec.push(Message {
                role: "user".to_string(),
                content: insert.clone(),
            });
            fs::write("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/insert1.txt", &insert).expect("unable to write file");
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": init_vec
            });
            init_vec.pop();
            println!("Still going through the loop");

            let cum_str = send_groq_api_request(self.groq_key.clone(), request_body);

            match cum_str.await {
                Ok(response_text) => {
                    let (finished, unfinished) = parse_topics(&response_text);
                    f = unfinished;
                    let mut unfinished_write = OpenOptions::new() //this is all for debugging purposes
                        .write(true)
                        .append(true)
                        .open("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/unfinished.txt")
                        .expect("unable to open file");
                    unfinished_write.write_all(f.as_bytes()).expect("unable to write file");

                    let middle = format!("{:?}\n\n",finished);
                    let mut finished_write = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/finished.txt")
                        .expect("unable to open file");
                    finished_write.write_all(middle.as_bytes()).expect("unable to write file");

                    for i in &finished {
                        total.push(i.clone());
                    }
                }
                Err(error_message) => {
                    eprintln!("Error: {}", error_message);
                }
            }
        }
        println!("finished tagging the text into the topic chunks");
        let mut llm = String::new();
        let mut stored_vec = prompts::MINOR.to_vec();
        for items in total{
            stored_vec.push(Message {
                role: "user".to_string(),
                content: items,
            });
            let write_string = format!("{:?}\n\n\n", stored_vec);

            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .open("/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/whatIsHappenning.txt")
                .expect("unable to open file");
            file.write_all(write_string.as_bytes()).expect("unable to write file");

            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": stored_vec.clone()
            });
            stored_vec.pop();


            let final_output = send_groq_api_request(self.groq_key.clone(), request_body);

            match final_output.await {
                Ok(response_text) => {
                    // Write the response_text to the file
                    fs::write(
                        "/Users/j-supha/Desktop/Personal_AI/FFMPEG/audio/src/parsing/google_docs.txt",
                        &response_text,
                    )
                    .expect("unable to write file");
                    
                    // Append the response_text to llm
                    llm = format!("{}\n\n{}", llm, response_text);
                }
                Err(error_message) => {
                    eprintln!("Error: {}", error_message);
                    // Handle the error case
                }
            }

            
        }
      
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
        self.rag_tag(text).await?;
        println!("Finished RAG_TAGGING the text...");
        Ok(())
    }
}