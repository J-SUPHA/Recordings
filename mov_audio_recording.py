import subprocess 
import sys 

def convert_video_to_audio(video_file, output_file='output_audio.wav'):
    """convert video file to audio using ffmpeg"""
    commnd = ["ffmpeg", "-i", video_file, "-vn", "-acodec", "pcm_s161e", "ar", "16000", "-ac", "1", output_file]
    subprocess.run(command, check=True)
    return audio_file

def transcribe_audio(audio_file, model_path="/Users/j-supha/FFMPEG/whisper.cpp/models/ggml-base.en.bin"):
    """transcribe audio file using whisper"""
    command = f"/Users/j-supha/FFMPEG/whisper.cpp/main --model {model_path} --output-txt {audio_file}"
    process = subprocess.Popen(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output, error = process.communicate()
    if output:
        print(output.decode())
        return f"{audio_file}.txt"
    else: 
        print(error.decode())
        return None

        
def post_process(transcription_file, max_length=14000):
    """Process transacription file to summarize the text."""
    with open(transcription_file, "r") as file:
        content = file.read()

    chunks = [content[i:i+max_length] for i in range(0, len(content) , max_length)]
    for i, chunk in enumerate(chunks):
        response = ollama.chat(model="llama3", messages=[
            {
                "role": "system",
                "content": "You are a text summarizer. You will be given plain text that may be coming from multiple individuals. Your job is to summarize the incoming conversation into a understandable paragraph of text that summarizes what happened."
            }, 
            {
                "role": "user",
                "content": chunk
            }
        ])
        with open("final_output.txt", "a") as file:
            file.write(response['message']['content'])


if __name__ == '__main__':
    if len(sys.argv) < 2 :
        print("Usage: python3 mov_audio_recording.py <video_file>")
        sys.exit(1)
    video_file = sys.argv[1]
    audio_file = convert_video_to_audio(video_file)
    transcription_file = transcribe_audio(audio_file)

    with open(transcription_file, "w") as file:
        file.write(transacription_text)
    post_process(transcription_file)


