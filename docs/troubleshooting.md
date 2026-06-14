# Troubleshooting

- Preview requires a default system output device.
- Batch processing currently supports stable WAV input and WAV output.
- Compressed formats may preview if Symphonia can decode them locally, but v0.1 export is WAV-only.
- `amixima validate` uses strict Soundcourse parsing. If an older INI file loads in the TUI but fails validation, add every required parameter explicitly.
- Batch processing only scans top-level WAV files in the selected directory.
