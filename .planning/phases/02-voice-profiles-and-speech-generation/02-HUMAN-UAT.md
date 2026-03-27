---
status: partial
phase: 02-voice-profiles-and-speech-generation
source: [02-VERIFICATION.md]
started: 2026-03-28T00:00:00Z
updated: 2026-03-28T00:00:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. End-to-end design command
expected: Run `chatter design 'warm friendly male voice'` — profile saved in ~/.config/chatter/profiles/ with sample.mp3 and voice_prompt.bin (or ref_audio.wav on MLX)
result: [pending]

### 2. Clone command with real audio
expected: Run `chatter clone reference.mp3` — profile saved with sample.mp3 and voice_prompt.bin; check language storage format (full name vs short code)
result: [pending]

### 3. Generate with saved profile
expected: Run `chatter generate 'Hello world' --profile myvoice` — MP3 file written to current directory, file is audible speech
result: [pending]

### 4. Profiles list table rendering
expected: Run `chatter profiles list` after creating profiles — table shows name, type, language, and creation date columns
result: [pending]

### 5. Language override (GEN-06)
expected: Run `chatter generate 'Hello' --profile myvoice --language english` — speech generated in English regardless of profile's stored language
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
