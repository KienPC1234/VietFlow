# Audio assets

`notification.wav` is the embedded V/E notification chime.

- Format: WAV, IMA ADPCM (4-bit), mono, 22,050 Hz, 0.16 s
- Size: 2,142 bytes
- Source: generated locally from soft C5/G5 tones (523.25 Hz / 783.99 Hz), low amplitude, 8 ms attack and 20 ms fade-out to avoid clicks and harshness
- Embedded by `platforms/windows/src/hook.rs`; no external file or system sound scheme is required

Regenerate with ffmpeg:

```powershell
ffmpeg -y -i notification_pcm.wav -c:a adpcm_ima_wav notification.wav
```
