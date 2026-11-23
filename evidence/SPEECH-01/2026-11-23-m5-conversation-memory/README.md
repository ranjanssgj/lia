# SPEECH-01 — Synthetic Speech Provider, Push-to-Talk Transcription, and TTS Duration Bounds

**Gate:** SPEECH-01 (partial implementation checks).  
**Gate Status:** **Not Run**. This run validates local synthetic speech provider ASR transcription, audio token validation, corrupted audio rejection, and deterministic TTS speech duration bounds against synthetic agent fixtures; full conformance gate remains **Not Run** until live local ASR/TTS runtime integration on the pinned Linux profile.  
**Date:** 23 November 2026.  
**Procedure Revision:** P5 M5 task plan (`docs/p5/tasks/M5-conversation-memory.md`).  
**Environment:** Linux 6.18.2-arch2-1 x86_64; rustc 1.89.0 (pinned); rusqlite 0.41.1; SQLite 3.54.1.

## Scope and Invariants Validated
1. **Push-to-Talk ASR Transcription (AC-002):** Speech tokens (e.g. `token://voice/hello_lia`, `token://voice/move_window`) are transcribed into structured text turns without exposing raw microphone audio to storage or logging.
2. **Corrupted Audio Token Rejection:** Malformed or corrupted voice input tokens are rejected with explicit speech errors, preventing erroneous transcription pipeline execution.
3. **Deterministic TTS Synthesis Estimation:** Synthetic text-to-speech provides realistic, bounded duration estimates (e.g., character count pacing with a minimum duration floor) to drive Godot character speech animation timers.
4. **Zero Remote Speech Egress:** Speech processing is strictly local; no audio bytes or transcripts are sent to third-party speech APIs.

## Raw Output and Checksums
- Raw test run: [`raw.txt`](raw.txt)
- Checksum file: [`raw.sha256`](raw.sha256)
- Checksum: `d5905f11c71e7eb4e9ce5403d56fc1d7b3427a6b7256a7903aaceec865d26ca1`

## Results
- Synthetic speech transcription and duration tests passed.
- Conformance gate SPEECH-01 status: **Not Run**.
