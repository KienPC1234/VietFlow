# Audio assets

`notification.wav` is the embedded V/E notification chime.

- Format: WAV, uncompressed 16-bit PCM, mono, 48,000 Hz, 0.075 s
- Size: 7,244 bytes
- Source: generated locally with smooth cosine envelope (4 ms attack, 4 ms release), -12 dBFS headroom, dual-tone C6/G6 bell (1046.50 Hz / 1567.98 Hz)
- Zero resampling or compression artifacts; eliminates clicking, buzzing, and distortion on Bluetooth LE (BLE) and A2DP wireless earphones
- Embedded by `platforms/windows/src/hook.rs`; requires no external ACM codecs, media files, or system sound schemes
