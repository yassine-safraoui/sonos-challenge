This file will contain my thoughts as I do the challenge, like hypothesis and explanation of various decisions.

# Wave File

the wave file that will be used is in data/song.wav
It is the famous Rick Roll song, I downloaded it from youtube using yt-dlp and then converted it to wav using ffmpeg:

```shell
yt-dlp -f bestaudio -o song.m4a https://www.youtube.com/watch?v=dQw4w9WgXcQ
ffmpeg -i song.m4a -ac 1 -ar 44100 song.wav
```

here are the specs of the wav file, those are obtained using `reader.spec()` method of hound:

```
spec: WavSpec { channels: 1, sample_rate: 44100, bits_per_sample: 16, sample_format: Int }
```

As you can see, there is only one channel in the file. I decided to keep one channel to simplify the sending of the wav
file over TCP

---

I added the num-traits crate to have an integer type to be used in AudioMessage enum for genericity reasons

---

I decided to decode the wav file on the server first and then reserialize it into a spec + samples format intentionally,
I'm aware that I could have streamed the wav file as is to the client and then read it there, but I didn't because:

- the clients would have to deal with corrupt wav files if the server encounters them, I prefer to keep this on the
  server side for the sake of simplicity. In addition, clients receive audio data to process it, so it doesn't make
  sense for the server to send wav data it didn't even verify. We could have first verified the correctness of the wav
  data and then sent it as a stream to the clients without having to do our own serialization, we didn't for the next
  reason.
- the server may receive audio data from sources other than a wav file, like a microphone; this will be handled with the
  CPAL crate, when looking at the CPAL crate, we can see that it provides a stream configuration that is similar to the
  WavConfig hound gives, so serializing that config along with the stream in the same way the Wav file is serialized
  would be basic and an advantage of this method, unlike converting the microphone stream into a wav stream which
  god knows how it should be done.

---

I decided to use i16 as the sample type for the audio samples because it's generally enough for most users. 24 bits and
32 bits would be overkill and are used only in professional settings.