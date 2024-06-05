import pyaudio
import wave
import subprocess
import time
import ollama
import threading

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
        self.running = threading.Event()
        self.last_processed_index = 0
        self.lock = threading.Lock()
        self.recording_thread = None  # Thread management

    def signal_handler(self, sig, frame):
        print("Stopping recording")
        self.stop_recording()

    def stop_recording(self):
        print("Stopping the recording")
        if len(self.frames) > self.last_processed_index:
            filename = self.save_frames(int(time.time()))
            self.process_audio_file(filename)
        try:
            self.running.clear()
            print("running flag cleared")
        except Exception as e:
            print(f"Error clearing the running flag: {e}")

        try:
            self.stream.stop_stream()
            self.stream.close()
            print("STREAM CLOSED!!!!!")
        except Exception as e:
            print(f"Error stopping the stream: {e}")
        try:
            print("AUDIO TERMINATED!!!")
            self.audio.terminate()
            print("audio terminated")
        except Exception as e:
            print(f"Error terminating PyAudio: {e}")
        print("HIT HERE")
        if self.recording_thread:
                self.recording_thread.join(timeout=5)  # Wait for the thread to finish with a timeout
                if self.recording_thread.is_alive():
                    print("Warning: Recording thread did not terminate as expected.")
                else:
                    self.recording_thread = None
        print("Finished Recording")

    
    def callback(self, in_data, frame_count, time_info, status):
        if not self.running.is_set():
            return (None, pyaudio.paComplete)
        self.frames.append(in_data)
        return (in_data, pyaudio.paContinue)

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

        for chunk in chunks:
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
        with self.lock:
            try:
                self.stream = self.audio.open(format=pyaudio.paInt16, channels=self.channels, rate=self.rate, input=True, frames_per_buffer=self.chunk)
                print("Recording started...")
                self.running.set()
            except Exception as e:
                print(f"Error starting the recording: {e}")
                return
            try:
                while self.running.is_set():
                    if self.stream.is_active():
                        data = self.stream.read(self.chunk, exception_on_overflow=False)
                        self.frames.append(data)
                    else:
                        print("Stream inactive, stopping...")
                        break
                    print("print the status ", self.running.is_set())
            finally:
                # print("this actually ran properly")
                # if hasattr(self, 'stream') and self.stream.is_active():
                #     self.stream.stop_stream()
                #     self.stream.close()
                # self.audio.terminate()
                print("Recording stopped and resources cleaned up.")


if __name__ == "__main__":
    print("Enter the entry point")
    recorder = AudioRecorder()

    def manage_input():
        while True:
            cmd = input("Type 'start' to start recording, 'stop' to stop recording, and 'exit' to exit: ").strip().lower()
            if cmd == 'start':
                if not recorder.running.is_set():
                    print("Attempting to start recording...")
                    recorder.recording_thread = threading.Thread(target=recorder.start_recording)
                    recorder.recording_thread.start()
                else:
                    print("Recording is already running.")
            elif cmd == 'stop':
                if recorder.running.is_set():
                    recorder.stop_recording()
                else:
                    print("Recording is not active.")
            elif cmd == 'exit':
                if recorder.running.is_set():
                    recorder.stop_recording()
                print("Exiting the program.")
                break

    input_thread = threading.Thread(target=manage_input)
    input_thread.start()
    input_thread.join()  # Wait for the input thread to finish before exiting the program
    print("Program terminated.")

