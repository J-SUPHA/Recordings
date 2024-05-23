import pyaudio
import wave
import subprocess
import signal
import time
import ollama

# run this to record your microphone audio record it and then process it 


class AudioRecorder:
    def __init__(self, channels=1, rate=16000, chunk=1024, record_seconds=60, model_path="/Users/j-supha/FFMPEG/whisper.cpp/models/ggml-base.en.bin"):
        self.channels = channels
        self.rate = rate
        self.chunk = chunk
        self.record_seconds = record_seconds
        self.model_path = model_path
        self.audio = pyaudio.PyAudio()
        self.frames = []
        self.running = True
        self.last_processed_index = 0
        self.setup_signal_handling()

    def setup_signal_handling(self):
        signal.signal(signal.SIGINT, self.signal_handler)
        signal.signal(signal.SIGTERM, self.signal_handler)

    def signal_handler(self, sig, frame):
        print("Stopping recording")
        self.stop_recording()

    def stop_recording(self):
        print("Stopping the recording")
        if len(self.frames) > self.last_processed_index:
            filename = self.save_frames(int(time.time()))
            self.process_audio_file(filename)
        self.stream.stop_stream()
        self.stream.close()
        self.audio.terminate()
        print("Finished Recording")

    def save_frames(self, filename_suffix):
        print("Saving the frames")
        filename = f"output_{filename_suffix}.wav"
        wavefile = wave.open(filename, 'wb')
        wavefile.setnchannels(self.channels)
        wavefile.setsampwidth(self.audio.get_sample_size(pyaudio.paInt16))
        wavefile.setframerate(self.rate)
        wavefile.writeframes(b''.join(self.frames[self.last_processed_index:]))
        wavefile.close()
        return filename

    def process_audio_file(self, filename):
        print(f"Processing file: {filename}")
        command = f"/Users/j-supha/FFMPEG/whisper.cpp/main --model {self.model_path} --output-txt {filename}"
        process = subprocess.Popen(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        output, error = process.communicate()
        if output:
            print(output.decode())
            self.post_process(f"{filename}.txt")

    def post_process(self, filename):
        with open(filename, 'r') as file:
            content = file.read()
        chunks = [content[i:i+14000] for i in range(0, len(content), 14000)]

        with open("final_output.txt", 'w') as file:
            file.write("")  # Clear existing content

        for i,chunk in enumerate(chunks):
            response = ollama.chat(model="llama3", messages=[
                {
                    "role": "system",
                    "content": "You are a text summarizer. You wll be givne plain text that may be coming from multiple individuals. you job is summarize the incoming conversation into a understandable paragraph of text that summarizes what happened"
                },
                {
                    'role': 'user',
                    'content': chunk
                }
            ])
            if response and 'message' in response and 'content' in response['message']:
                with open("final_output.txt", 'a') as file:
                    file.write(response['message']['content'] + "\n")  # A



    def start_recording(self):
        print("Starting Recording")
        self.stream = self.audio.open(format=pyaudio.paInt16, channels=self.channels, rate=self.rate, input=True, frames_per_buffer=self.chunk)
        print("Recording...")
        try:
            while self.running:
                data = self.stream.read(self.chunk)
                self.frames.append(data)
        except Exception as e:
            print(f"Error: {e}")
            self.stop_recording()

if __name__ == "__main__":
    print("enter the entry point")
    recorder = AudioRecorder()
    recorder.start_recording()
    print("Recording Finished")
