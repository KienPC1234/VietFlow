# Audio assets

`notification.wav` is the embedded V/E notification chime.

- Format: WAV, IMA ADPCM (4-bit), mono, 22,050 Hz, 0.24 s
- Size: 3,166 bytes
- Source: generated locally from two short sine tones (880 Hz / 1174.66 Hz) with a soft attack and decay
- Embedded by `platforms/windows/src/hook.rs`; no external file or system sound scheme is required

Regenerate with ffmpeg:

```powershell
ffmpeg -y -i notification_pcm.wav -c:a adpcm_ima_wav notification.wav
```
