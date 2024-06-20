use portaudio as pa;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use crate::error::AppError;
use crate::parsing::Sst;

pub struct AudioRecorder {
    stream: pa::Stream<pa::NonBlocking, pa::Input<i16>>,
    pub frames: Arc<Mutex<Vec<i16>>>,
    pub notify: Arc<Notify>,
}

const SAMPLE_RATE: f64 = 16000.0;
const FRAMES_PER_BUFFER: u32 = 1024;
const CHANNELS: i32 = 1;


impl AudioRecorder {
    pub fn new(pa_handle: &pa::PortAudio) -> Result<Self, AppError> {
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

    pub fn start(&mut self) -> Result<(), pa::Error> {
        self.stream.start()
    }

    pub async fn stop(&mut self, recording: String) -> Result<(), AppError> {
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
        let mut tts_llm = Sst::new(filename.clone(), "/Users/j-supha/Desktop/Personal_AI/FFMPEG/whisper.cpp/models/ggml-base.en.bin".to_string());

        tts_llm.process_audio_file().await?;
            
        Ok(())
    }
}
