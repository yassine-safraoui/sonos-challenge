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
Note: I removed it later because I decided to use i16 as the sample type
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

---
I added iter_samples to the WavAudioSource enum which returns an iterator over the samples of the audio source, this is
useful when we don't want to read all the samples at once. It also sets the stage for when I add MicrophoneAudioSource
which will probably return a stream of samples.
I will also be using streams for network since they're more suitable for this application. That is because we will be
sending data to the clients continuously, so it's better to avoid reading all the data at once and do that in a loop.

---

Changed my mind, I'm not touching streams for the server. For this challenge, it's not a big deal if the client blocks
when waiting for new audio messages, it wouldn't have anything to do if it does not have audio messages anyway.

On the server, I'm creating a listener to accept connections from the client, I'm not handling shutdown of the listener,
I just move the listener to a new thread and make it accept all incoming connections and i forget about it. The only way
to stop the listener is to stop the server. For this challenge, this is acceptable.

---
I just realized a bunch of things, they're relatively a lot, i'll put them in the spec-change-and-jitter.md file.
Note: the file may be outdated since i changed my mind and decided to have a simple set new client message approach
instead of doing a callback.

---
I improved the error handling in tcp by ensuring the thread handling new clients doesn't panic because of mutexes.

---
I acknowledge that I'm cloning samples_group everytime I want to send it to the clients, this is inefficient. However,
fixing it would require using two structs AudioMessageOwned and AudioMessageRef or using the Cow type, and I don't want
to do either. For the sake of simplicity, I'm keeping the cloning approach,
it only allocates 2KB(1000 * two bytes of i16) of memory each time, and the memory is freed right after so it shouldn't
be a big deal.

---
I solved audio playback jitter, it was caused by two issues, I was incorrectly playing a mono source as stereo, and I
was not handling sleeping on the server well. waiting a wait time linked to the sample rate would make tcp latencies
cause audio playback issues, multiplying it by 0.8 solves the issue but will make the reception buffer slowly fill over
time and it will eventually get full. to solve this I first at the start of the server send the first 3 seconds of
playback and then start sending samples at the normal rate, this way the samples sent first will compensate for any tcp
latency.