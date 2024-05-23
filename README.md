ffmpeg -i Checking.mov -vn -acodec pcm_s16le -ar 16000 fin.wav

i specifies the input file for ffmpeg. In this case checking.mov is the input file. This is likely a Quicktime moe file

-acodec pcm_s16le This option specfies the audi codec to use for the output file. pcm_s161e stands for pulse code modulation
Signed 16-bit Little Endan. PCM is a standard way of encoding audio as digital values, and its typically used for high
quality audio in WAV files. The signed 16-bit part indicates that the audio smaples are rpresented uisng 16 bits per sample with a sign. Little endian refers to the byte order used to rpresent these 16 bit values. 

-ar 16000 the ar flag specifies the the audo sample rate for the output file in this case 16000 Hertz. 


remeber the whsiper command is ./main -m models/ggml-base.en.bin ../fin.wav 
Supoer simple anyone could do it really 