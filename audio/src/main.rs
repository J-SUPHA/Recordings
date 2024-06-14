use portaudio as pa;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use std::io::{self, Write};
use std::process::Command;
use std::fs;
use std::fs::File;
use serde::{Deserialize, Serialize};
use std::fmt;
use serde_json::Error as SerdeJsonError;
use std::path::Path;

extern crate reqwest;
use reqwest::Client;
// use serde_json::json;

const SAMPLE_RATE: f64 = 16000.0;
const FRAMES_PER_BUFFER: u32 = 1024;
const CHANNELS: i32 = 1;


#[derive(Debug)]
enum AppError {
    IoError(std::io::Error),
    PortAudioError(portaudio::Error),
    SerdeJsonError(SerdeJsonError),
    Other(String),  // For other types of errors
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter <'_>) -> fmt::Result {
        match self {
            AppError::IoError(e) => write!(f, "IO Error: {}", e),
            AppError::PortAudioError(e) => write!(f, "PortAudio Error: {}", e),
            AppError::Other(e) => write!(f, "Other Error: {}", e),
            AppError::SerdeJsonError(e) => write!(f, "Serde JSON Error: {}", e),
        }
    }
}
impl From<SerdeJsonError> for AppError {
    fn from(error: SerdeJsonError) -> Self {
        AppError::SerdeJsonError(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e)
    }
}
impl From<portaudio::Error> for AppError {
    fn from(e: portaudio::Error) -> Self {
        AppError::PortAudioError(e)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    model: String,
    created_at: String,
    message: Message,
    done: bool,
}

fn split_into_chunks(input: &str, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut iter = input.chars();

    while let Some(chunk) = iter.by_ref().take(chunk_size).collect::<String>().into() {
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
    }

    chunks
}

fn parse_topics(response: &str) -> (Vec<String>, String) {
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



#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

struct Sst {
    audio_file: String,
    model_path: String,
}

// the google doc API key that we need to use
//314667396760-lkfko65c43uej9en2d7vbu55nom5qqg9.apps.googleusercontent.com


impl Sst {
    fn new(audio_file: String, model_path: String) -> Self {
        Self {
            audio_file,
            model_path,
        }
    }

    fn extract_text_from_audio(&self) -> Result<String, AppError> {
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
    async fn send_text_to_api(&mut self, text: String) -> Result<(), AppError> {
        let client = Client::new();
        let my_vec = split_into_chunks(&text, 2000);
        let mut F = String::new();
        let mut total: Vec<String> = Vec::new();
        for vectors in my_vec {
            let insert = format!("{} {}", F,vectors);
            let request_body = serde_json::json!({
                "model": "llama3",
                "messages": [
                    {
                        "role": "system",
                        "content": "Your task is to group the following conversation between multiple parties by sub topic. You will be given a chunk of text and you have to group the text into a segments where each segment relates to a specific topic of conversation. I will give you a piece of text and you will insert <topic> tag at the beginning of the segment and the </topic> tag at the end of the segment. You need to return the entire segment with the tags in place. If you determine that the segment has not finished then do not add the final </topic> tag. Make sure that all the given text is encapsulated within a set of <topic> </topic> tags. You may encouter a case where the user has written a <topic> already. Just determine when that topic ends and then continue as normal"
                    },
                    {
                        "role": "user",
                        "content": "This User Agreement explains your rights and obligations in accessing, visiting and/or using the Service, brought to you by Condé Nast. This User Agreement does not apply to websites, apps, destinations, or other offerings that we do not own or control, even if they are linked to from the Service. All capitalized terms used in this User Agreement that are not otherwise defined have the meanings set forth in the Glossary.
                  You can access this User Agreement any time in the footer of the Service's home page, via the menu button/hamburger icon or on the Service description screen, or as otherwise indicated depending on the Service you are using. By purchasing a Product , registering for any aspect of the Service, or otherwise accessing, visiting or using the Service, you consent and agree to be bound by the terms of this User Agreement. If you do not agree with the terms and conditions of this User Agreement, you should not access, visit and/or use the Service, or request or receive a Product. We advise that you print or retain a digital copy of this User Agreement for future reference.
                  In addition to reviewing this User Agreement, please also review our Privacy Policy and any other terms and conditions that may be posted elsewhere in the Service or otherwise communicated to our users, because the Privacy Policy and all such terms and conditions are also part of the Agreement between you and us.
                  This User Agreement may be modified from time to time, so check back often. So that you are aware changes have been made, we will adjust the “Last Updated” date at the beginning of this document. If we make a material change to this User Agreement, we will also post on the Service a prominent notice that a change was made. Continued access, visitation and/or use of the Service by you, or continued receipt of a Product, will constitute your acceptance of any changes or revisions to the User Agreement.
                  ARBITRATION NOTICE AND CLASS ACTION WAIVER: EXCEPT FOR CERTAIN TYPES OF DISPUTES DESCRIBED IN SECTION VIII(G) BELOW, YOU AGREE THAT ALL DISPUTES BETWEEN YOU AND US WILL BE RESOLVED BY BINDING, INDIVIDUAL ARBITRATION AND YOU WAIVE YOUR RIGHT TO PARTICIPATE IN A CLASS ACTION LAWSUIT OR CLASS-WIDE ARBITRATION. READ MORE IN SECTION VIII(G) BELOW.
                  If you breach, violate, fail to follow, or act inconsistently with any part of the Agreement, we may terminate, discontinue, suspend, and/or restrict your account/profile, your ability to access, visit, and/or use the Service or any portion thereof, and/or the Agreement, including without limitation any of our purported obligations hereunder, with or without notice, in addition to our other remedies. In addition, we may curtail, restrict, or refuse to provide you with any future access, visitation, and/or use of the Service or any Product. We reserve the right, in addition to our other remedies, to take any technical, legal, and/or other action(s) that we deem necessary and/or appropriate, with or without notice, to prevent violations and enforce the Agreement and remediate any purported violations. You acknowledge and agree that we have the right hereunder to an injunction without posting a bond to stop or prevent a breach or violation of your obligations under the Agreement.
                  In the event of any conflict or inconsistency between the terms and conditions of this User Agreement, and any other terms and/or conditions applicable to the Service, we shall determine which rules, restrictions, limitations, terms and/or conditions shall control and prevail in our sole discretion, and you specifically waive any right to challenge or dispute such determination.
                  Monitoring. We strive to provide an enjoyable online experience for our users, so we may monitor activity on the Service to foster compliance with the Agreement. You hereby specifically agree to such monitoring. Nevertheless, we do not make any representations, warranties, covenants or guarantees that: (1) the Service, or any portion thereof, will be monitored for accuracy or unacceptable use, (2) apparent statements of fact will be authenticated, or (3) we will take any specific action (or any action at all) in the event of a challenge or dispute regarding compliance or non-compliance with the Agreement. We generally do not pre-screen Content before it is posted, uploaded, transmitted, sent or otherwise made available on or through the Service by users, so you may be exposed to Content that is opinionated, offensive, and/or inappropriate, including Content that violates the Agreement.
                  What to Do if You Have a Complaint Against Another User
                  Remember that by using the publicly accessible portions of the Service you may be exposed to Content that is opinionated, offensive, and/or inappropriate, including Content that may violate the Agreement. You should understand that not all of such Content is actionable. You may not use the Service, or lodge complaints against other users, to facilitate a personal dispute. If you have a legitimate complaint about another user, please do the following:
                  Harassment: If you have reason to believe that another person is using the Service in a way that is harmful to you or others (e.g., to impersonate or imitate you, or to stalk, bully, threaten, intimidate or otherwise harass you or others), we urge you to contact your local authorities, or appropriate state or federal agencies.
                  Copyright Complaints: If you have reason to believe that your Content has been copied and/or is accessible on the Service in a way that constitutes copyright infringement, or that the Service contains links or other references to another site, application, destination or service that contains Content or activity that infringes your copyright rights, you may notify us as described below in the section entitled Copyright Complaints.
                  CyberCrime: If you have reason to believe you may be the victim of an online crime, such as identity theft, fraud, infringement, or hacking, you may contact the Internet Crime Complaint Center, at www.ic3.gov, a partnership between the Federal Bureau of Investigation (FBI), the National White Collar Crime Center (NW3C), and the Bureau of Justice Assistance (BJA).
                  IMPORTANT: FALSE OR INACCURATE ACCUSATIONS THAT OTHERS HAVE COMMITTED A CRIME, INAPPROPRIATE ACT, OR VIOLATION OF OUR AGREEMENT, COULD BE A VIOLATION OF CRIMINAL AND/OR CIVIL LAW, OR OTHERWISE EXPOSE YOU TO LIABILITY FOR DAMAGES (INCLUDING COSTS AND ATTORNEYS' FEES).
                  "
                    },
                    {
                        "role": "assistant",
                        "content": "<topic>This User Agreement explains your rights and obligations in accessing, visiting and/or using the Service, brought to you by Condé Nast. This User Agreement does not apply to websites, apps, destinations, or other offerings that we do not own or control, even if they are linked to from the Service. All capitalized terms used in this User Agreement that are not otherwise defined have the meanings set forth in the Glossary.
                  You can access this User Agreement any time in the footer of the Service's home page, via the menu button/hamburger icon or on the Service description screen, or as otherwise indicated depending on the Service you are using. By purchasing a Product , registering for any aspect of the Service, or otherwise accessing, visiting or using the Service, you consent and agree to be bound by the terms of this User Agreement. If you do not agree with the terms and conditions of this User Agreement, you should not access, visit and/or use the Service, or request or receive a Product. We advise that you print or retain a digital copy of this User Agreement for future reference.</topic>
                  <topic>In addition to reviewing this User Agreement, please also review our Privacy Policy and any other terms and conditions that may be posted elsewhere in the Service or otherwise communicated to our users, because the Privacy Policy and all such terms and conditions are also part of the Agreement between you and us.</topic>
                  <topic>This User Agreement may be modified from time to time, so check back often. So that you are aware changes have been made, we will adjust the “Last Updated” date at the beginning of this document. If we make a material change to this User Agreement, we will also post on the Service a prominent notice that a change was made. Continued access, visitation and/or use of the Service by you, or continued receipt of a Product, will constitute your acceptance of any changes or revisions to the User Agreement.</topic>
                  <topic>ARBITRATION NOTICE AND CLASS ACTION WAIVER: EXCEPT FOR CERTAIN TYPES OF DISPUTES DESCRIBED IN SECTION VIII(G) BELOW, YOU AGREE THAT ALL DISPUTES BETWEEN YOU AND US WILL BE RESOLVED BY BINDING, INDIVIDUAL ARBITRATION AND YOU WAIVE YOUR RIGHT TO PARTICIPATE IN A CLASS ACTION LAWSUIT OR CLASS-WIDE ARBITRATION. READ MORE IN SECTION VIII(G) BELOW.</topic>
                  <topic>If you breach, violate, fail to follow, or act inconsistently with any part of the Agreement, we may terminate, discontinue, suspend, and/or restrict your account/profile, your ability to access, visit, and/or use the Service or any portion thereof, and/or the Agreement, including without limitation any of our purported obligations hereunder, with or without notice, in addition to our other remedies. In addition, we may curtail, restrict, or refuse to provide you with any future access, visitation, and/or use of the Service or any Product. We reserve the right, in addition to our other remedies, to take any technical, legal, and/or other action(s) that we deem necessary and/or appropriate, with or without notice, to prevent violations and enforce the Agreement and remediate any purported violations. You acknowledge and agree that we have the right hereunder to an injunction without posting a bond to stop or prevent a breach or violation of your obligations under the Agreement.</topic>
                  <topic>In the event of any conflict or inconsistency between the terms and conditions of this User Agreement, and any other terms and/or conditions applicable to the Service, we shall determine which rules, restrictions, limitations, terms and/or conditions shall control and prevail in our sole discretion, and you specifically waive any right to challenge or dispute such determination.</topic>
                  <topic>Monitoring. We strive to provide an enjoyable online experience for our users, so we may monitor activity on the Service to foster compliance with the Agreement. You hereby specifically agree to such monitoring. Nevertheless, we do not make any representations, warranties, covenants or guarantees that: (1) the Service, or any portion thereof, will be monitored for accuracy or unacceptable use, (2) apparent statements of fact will be authenticated, or (3) we will take any specific action (or any action at all) in the event of a challenge or dispute regarding compliance or non-compliance with the Agreement. We generally do not pre-screen Content before it is posted, uploaded, transmitted, sent or otherwise made available on or through the Service by users, so you may be exposed to Content that is opinionated, offensive, and/or inappropriate, including Content that violates the Agreement.</topic>
                  <topic>What to Do if You Have a Complaint Against Another User
                  Remember that by using the publicly accessible portions of the Service you may be exposed to Content that is opinionated, offensive, and/or inappropriate, including Content that may violate the Agreement. You should understand that not all of such Content is actionable. You may not use the Service, or lodge complaints against other users, to facilitate a personal dispute</topic>. <topic>If you have a legitimate complaint about another user, please do the following:
                  Harassment: If you have reason to believe that another person is using the Service in a way that is harmful to you or others (e.g., to impersonate or imitate you, or to stalk, bully, threaten, intimidate or otherwise harass you or others), we urge you to contact your local authorities, or appropriate state or federal agencies.
                  Copyright Complaints: If you have reason to believe that your Content has been copied and/or is accessible on the Service in a way that constitutes copyright infringement, or that the Service contains links or other references to another site, application, destination or service that contains Content or activity that infringes your copyright rights, you may notify us as described below in the section entitled Copyright Complaints.
                  CyberCrime: If you have reason to believe you may be the victim of an online crime, such as identity theft, fraud, infringement, or hacking, you may contact the Internet Crime Complaint Center, at www.ic3.gov, a partnership between the Federal Bureau of Investigation (FBI), the National White Collar Crime Center (NW3C), and the Bureau of Justice Assistance (BJA).</topic>
                  <topic>IMPORTANT: FALSE OR INACCURATE ACCUSATIONS THAT OTHERS HAVE COMMITTED A CRIME, INAPPROPRIATE ACT, OR VIOLATION OF OUR AGREEMENT, COULD BE A VIOLATION OF CRIMINAL AND/OR CIVIL LAW, OR OTHERWISE EXPOSE YOU TO LIABILITY FOR DAMAGES (INCLUDING COSTS AND ATTORNEYS' FEES).</topic>
                  "
                     },
                     {
                        "role": "user",
                        "content": "This bulletin provides basic information based on New Jersey statutory laws and case law regarding establishing and breaking leases for residential rental properties in New Jersey. This bulletin is for informational purposes only and should not be used for legal interpretations or legal advice. Please consult an attorney for legal services and advice when necessary.
                  A lease is an agreement between a lessor (landlord) and a lessee (tenant) which may be verbal or written. A lease grants possession to the tenant for use of a dwelling unit for a specified period of time in return for rent. A lease is considered a contract and must be written in plain language. This means that the lease must be written so that the average person can understand it. Parties to a lease must be at least 18 years of age and mentally competent. A written lease does not take effect until it is signed by the lessor
                  Lease terms begin on the date specified in the lease agreement. If the beginning date of the lease is not specified, the term will begin from the time the lease was dated. If the lease is not dated the term will begin when the lease is delivered. If the lease is verbal the term will began on any day agreed upon by the parties to the lease. There is no limitation to the length of the term of the lease. If a lease is for a term of more than three years it must be written, pursuant to N.J.S.A. 25:1-12. The landlord may not unilaterally change the terms of the lease agreement while there is a written lease in effect. If a new landlord acquires a rental property with a tenant, the new landlord must honor any existing lease agreement. Once the lease expires the landlord may make reasonable changes to the lease. Any changes to a written lease must be in writing and accepted by all parties.
                  Before signing a lease, tenants should read it thoroughly to be sure they understand it and agree with the terms of the lease. There is an attorney review period allowed for leases that are prepared by Real Estate Brokers or Salespersons licensed by the New Jersey Real Estate Commission. Either party may have an attorney review the lease. The attorney review period must be completed within three business days from the delivery of the lease to the tenant and landlord. Unless an attorney disapproves of the lease it will become legally binding after the attorney review period.
                  A landlord must allow the tenant to renew the lease unless the landlord has good cause for an eviction under the Anti-Eviction Act. (This does not apply to two or three-family owner occupied dwellings, motels, hotels, transients or seasonal tenants). Yearly and month-to-month leases will automatically renew for another term unless a valid notice to quit is given by the landlord or unless the tenant gives notice to the landlord that the tenant will return possession of the premises to the landlord.
                  If the tenant or landlord does not renew the lease and the lease was for a term of more than one month and the tenant holds over (stays after the expiration of the lease), the tenancy will become a month",
                     },
                     {
                        "role": "assistant",
                        "content": "<topic>This bulletin provides basic information based on New Jersey statutory laws and case law regarding establishing and breaking leases for residential rental properties in New Jersey. This bulletin is for informational purposes only and should not be used for legal interpretations or legal advice. Please consult an attorney for legal services and advice when necessary.</topic>
                  <topic>A lease is an agreement between a lessor (landlord) and a lessee (tenant) which may be verbal or written. A lease grants possession to the tenant for use of a dwelling unit for a specified period of time in return for rent. A lease is considered a contract and must be written in plain language. This means that the lease must be written so that the average person can understand it. Parties to a lease must be at least 18 years of age and mentally competent. A written lease does not take effect until it is signed by the lessor</topic>
                  <topic>Lease terms begin on the date specified in the lease agreement. If the beginning date of the lease is not specified, the term will begin from the time the lease was dated. If the lease is not dated the term will begin when the lease is delivered. If the lease is verbal the term will began on any day agreed upon by the parties to the lease. There is no limitation to the length of the term of the lease. If a lease is for a term of more than three years it must be written, pursuant to N.J.S.A. 25:1-12. The landlord may not unilaterally change the terms of the lease agreement while there is a written lease in effect. If a new landlord acquires a rental property with a tenant, the new landlord must honor any existing lease agreement. Once the lease expires the landlord may make reasonable changes to the lease. Any changes to a written lease must be in writing and accepted by all parties.</topic>
                  <topic>Before signing a lease, tenants should read it thoroughly to be sure they understand it and agree with the terms of the lease. There is an attorney review period allowed for leases that are prepared by Real Estate Brokers or Salespersons licensed by the New Jersey Real Estate Commission. Either party may have an attorney review the lease. The attorney review period must be completed within three business days from the delivery of the lease to the tenant and landlord. Unless an attorney disapproves of the lease it will become legally binding after the attorney review period.</topic>
                  <topic>A landlord must allow the tenant to renew the lease unless the landlord has good cause for an eviction under the Anti-Eviction Act. (This does not apply to two or three-family owner occupied dwellings, motels, hotels, transients or seasonal tenants). Yearly and month-to-month leases will automatically renew for another term unless a valid notice to quit is given by the landlord or unless the tenant gives notice to the landlord that the tenant will return possession of the premises to the landlord.</topic>
                  <topic>If the tenant or landlord does not renew the lease and the lease was for a term of more than one month and the tenant holds over (stays after the expiration of the lease), the tenancy will become a month
                  "
                     },
                    {
                        "role": "user",
                        "content": insert
                    }
                ]
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
                F = unfinished;
                for i in &finished {
                    total.push(i.clone());
                }
            }else{
                eprintln!("Failed to send request: {}", res.status());
            }
        }
        let mut llm = String::new();
        for items in total{
            let mut request_body = serde_json::json!({
                "model": "llama3",
                "messages": [
                    {
                        "role": "system",
                        "content": "Your  to take this conversation and sumarize in a clear and concise manner "
                    },
                    {
                        "role": "user",
                        "content": items
                    }
                ]
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
                llm = format!("{:?}\n\n{:?}",llm, cum_str)
            }else{
                eprintln!("Failed to send request: {}", res.status());
            }

        }
        println!("This is the culmination of some hard work");
            let _output = Command::new("python3")
                .arg("google_docs.py")  // Path to the Python script
                .arg("--write")
                .arg(F)               // Argument to pass to the Python script
                .output()                   // Executes the command as a child process
                .expect("Failed to execute command");
            // 1HFD4EzZqm_i_AUn3NcbI1Bz8rZNRpENqQuB4oNGmbKY this is the document ID

        Ok(())
    }

    // Combined method to process audio file and handle API interaction
    async fn process_audio_file(&mut self) -> Result<(), AppError> {
        let text = self.extract_text_from_audio()?;
        self.send_text_to_api(text).await?;
        Ok(())
    }

}

struct AudioRecorder {
    stream: pa::Stream<pa::NonBlocking, pa::Input<i16>>,
    frames: Arc<Mutex<Vec<i16>>>,
    notify: Arc<Notify>,
}

impl AudioRecorder {
    fn new(pa_handle: &pa::PortAudio) -> Result<Self, AppError> {
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

    async fn stop(&mut self, recording: String) -> Result<(), AppError> {
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

        tts_llm.process_audio_file().await?;
            
        Ok(())
    }
}


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
                    self.full_pipeline().await;
                }
                "audio" => {
                    self.text_file().await;
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
            println!("Type the path to the wav file that you want to change. If the wav file is not found you will be asked to type it again. Type exit to exit:\n");
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
            println!("File found. Processing audio file...");
            let mut sst = Sst::new("forge_meets.wav".to_string(), "/Users/j-supha/FFMPEG/whisper.cpp/models/ggml-base.en.bin".to_string());
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


