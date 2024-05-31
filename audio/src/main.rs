use portaudio as pa;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use std::io::{self, Write, Error};
use std::process::Command;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use serde::{Deserialize, Serialize};

extern crate reqwest;
use reqwest::Client;
// use serde_json::json;

const SAMPLE_RATE: f64 = 16000.0;
const FRAMES_PER_BUFFER: u32 = 1024;
const CHANNELS: i32 = 1;


#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    model: String,
    created_at: String,
    message: Message,
    done: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

struct Sst {
    audio_file: String,
    model_path: String,
}


//314667396760-lkfko65c43uej9en2d7vbu55nom5qqg9.apps.googleusercontent.com


impl Sst {
    fn new(audio_file: String, model_path: String) -> Self {
        Self {
            audio_file,
            model_path,
        }
    }

    fn extract_text_from_audio(&self) -> Result<String, Error> {
        let output_txt = format!("{}.txt", self.audio_file);
        let command = format!("/Users/j-supha/FFMPEG/whisper.cpp/main --model {} --output-txt {} {}", self.model_path, output_txt, self.audio_file);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()?;

        if !output.stdout.is_empty() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }

        // Read the output from the text file
        let text = fs::read_to_string(output_txt)?;
        Ok(text)
    }

    // Method to send extracted text to API and handle response
    async fn send_text_to_api(&mut self, text: String) -> Result<(), Error> {
        let client = Client::new();
        let request_body = serde_json::json!({
            "model": "llama3",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a text summarizer. You will summarize incoming text."
                },
                {
                    "role": "user",
                    "content": text
                }
            ]
        });
    
        let mut res = client.post("http://localhost:11434/api/chat")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
    
        if res.status().is_success() {
            println!("Request successful: {}", res.status());
            
            let mut file = File::create("output.txt")?;
            
            while let Some(chunk) = res.chunk().await.map_err(|e| io::Error::new(ErrorKind::Other, e))? {
                let api_response: ApiResponse = serde_json::from_slice(&chunk)?;
                println!("Message Content: {}", api_response.message.content);
                file.write_all(api_response.message.content.as_bytes())?;
            }
            
            file.sync_all()?;
        } else {
            eprintln!("Failed to send request: {}", res.status());
        }
    
        Ok(())
    }

    // Combined method to process audio file and handle API interaction
    async fn process_audio_file(&mut self) -> Result<(), Error> {
        let text = self.extract_text_from_audio()?;
        self.send_text_to_api(text).await
    }

}

struct AudioRecorder {
    stream: pa::Stream<pa::NonBlocking, pa::Input<i16>>,
    frames: Arc<Mutex<Vec<i16>>>,
    notify: Arc<Notify>,
}

impl AudioRecorder {
    fn new(pa_handle: &pa::PortAudio) -> Result<Self, pa::Error> {
        let settings = pa_handle.default_input_stream_settings(
            CHANNELS,
            SAMPLE_RATE,
            FRAMES_PER_BUFFER,
        )?;
        let frames = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(Notify::new());

        let stream_frames = frames.clone();
        let stream_notify = notify.clone();

        let callback = move |pa::InputStreamCallbackArgs { buffer, .. }| {
            let mut frames = stream_frames.lock().unwrap();
            frames.extend_from_slice(buffer);
            stream_notify.notify_one();
            pa::Continue
        };

        let stream = pa_handle.open_non_blocking_stream(settings, callback)?;

        Ok(Self { stream, frames, notify })
    }

    fn start(&mut self) -> Result<(), pa::Error> {
        self.stream.start()
    }

    async fn stop(&mut self, recording: String) -> Result<(), pa::Error> {
        self.stream.stop()?; // Stop the audio stream

        println!("Recording stopped {:?}", recording);

        // Access the shared audio frames
        let mut frames = self.frames.lock().unwrap();

        // Define the path and specifications for the output WAV file
        let spec = hound::WavSpec {
            channels: CHANNELS as u16,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 16, // Assuming 16-bit samples
            sample_format: hound::SampleFormat::Int, // Integer format
        };

        // Create a new WAV writer
        let filename = format!("{}.wav", recording.trim());
        let mut writer = hound::WavWriter::create(filename.clone(), spec).unwrap();

        // Write each sample to the WAV file
        for &sample in frames.iter() {
            writer.write_sample(sample).unwrap();
        }

        // Finalize the WAV file
        writer.finalize().unwrap();

        // Clear the audio frames
        frames.clear();
        let mut tts_llm = Sst::new(filename.clone(), "/Users/j-supha/FFMPEG/whisper.cpp/models/ggml-base.en.bin".to_string());

        match tts_llm.process_audio_file().await {
            Ok(_) => println!("Audio file processed successfully."),
            Err(e) => println!("Failed to process audio file: {}", e),
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), pa::Error> {
    let pa = pa::PortAudio::new()?;

    let mut recorder = AudioRecorder::new(&pa)?;
    // recorder.start()?;

    let is_recording = Arc::new(Mutex::new(false));
    let mut recording_name = String::new();

    let recorder_frames = recorder.frames.clone();
    let recorder_notify = recorder.notify.clone();

    // // Spawning an asynchronous task within the Tokio runtime
    let _handle = tokio::spawn(async move {
        loop {
            // Wait for a notification asynchronously
            recorder_notify.notified().await;
            let _frames = recorder_frames.lock().unwrap();
        }
    });

    // Command line interface to control recording
    loop {
        println!("Type 'start' to start recording, 'stop' to stop recording, and 'exit' to exit: ");
        io::stdout().flush().unwrap();
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        

        let mut flag = is_recording.lock().unwrap(); 

        match command.trim() {
            "start" => {
                if *flag {
                    println!("Recording already in progress");
                    continue;
                }else {
                    println!("Enter name of the recording: ");
                    io::stdout().flush().unwrap();
                    io::stdin().read_line(&mut recording_name).unwrap();
                    *flag = true;
                    recorder.start()?;
                } 
            }
            "stop" => {
                if *flag {
                    recorder.stop(recording_name.clone()).await?;
                    recording_name.clear();
                    *flag = false;
                } else {
                    println!("No recording in progress");
                }
                
            }
            "exit" => {
                if *flag {
                    recorder.stop(recording_name.clone()).await?;
                    recording_name.clear();
                    *flag = false;
                }
                println!("Exiting...");
                break;
            }
            _ => println!("Invalid command"),
        }
    }


    // let mut sst = Sst::new("forge_meets.wav".to_string(), "/Users/j-supha/FFMPEG/whisper.cpp/models/ggml-base.en.bin".to_string());
    // match sst.process_audio_file().await {
    //     Ok(_) => println!("Audio file processed successfully."),
    //     Err(e) => println!("Failed to process audio file: {}", e),
    // }

    Ok(())
}


