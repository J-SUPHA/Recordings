mod audio;
mod parsing;
mod error;

use audio::AudioRecorder;
use parsing::Sst;
use error::AppError;

use portaudio as pa;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};


use std::path::Path;

extern crate reqwest;


struct Control {
    is_recording: bool,
}

impl Control {
    fn new() -> Self {
        Self {
            is_recording: false,
        }
    }

    async fn control(&mut self) -> Result<(), AppError>{
        loop {
            println!("full for the full pipeline, audio to process a wav file, and exit to exit:");
            io::stdout().flush().unwrap();
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            
            match command.trim() {
                "full" => {
                    let _ = self.full_pipeline().await;
                }
                "audio" => {
                    let _ = self.text_file().await;
                }
                "exit" => {
                    println!("Exiting...");
                    break;
                }
                _ => println!("Invalid command"),
            }
        }
        Ok(())

    }

    async fn full_pipeline(&mut self) -> Result<(), AppError> {
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
        Ok(())
    }

    async fn text_file(&mut self) -> Result<(), AppError> {
        loop {
            println!("Type the path to the wav file that you want to change. If the wav file is not found you will be asked to type it again. Type exit to exit:");
            io::stdout().flush().unwrap();
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            let command = command.trim();
            if command == "exit" {
                break;
            }
            if !command.ends_with(".wav") {
                println!("Invalid file type. Please enter a .wav file");
                continue;
            }
            let path = Path::new(command);
            if !path.exists() {
                println!("File not found. Please enter a valid path");
                continue;
            }
            println!("File found. Processing audio file {:?}", command);
            let mut sst = Sst::new(command.to_string(), "/Users/j-supha/Desktop/Personal_AI/FFMPEG/whisper.cpp/models/ggml-base.en.bin".to_string());
            sst.process_audio_file().await.map_err(|e| AppError::Other(e.to_string()))?;
            break;
        }
        Ok(())
    }
}


#[tokio::main]
async fn main() -> Result<(), AppError> {
    let mut flow = Control::new();
    flow.control().await?;
    Ok(())
}