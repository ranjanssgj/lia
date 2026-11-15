# P5 model and speech evaluation policy

**Status:** Evaluation policy approved for the P5 slice; no provider or model is selected.
**Source:** Owner-requested decision brief, 15 November 2026.
**Applies to:** MODEL-01, SPEECH-01 and later MEM-01.

## Data-egress rules

- Raw audio remains on the device and is transient. P5 does not use hosted ASR.
- S0 text may be sent to a hosted LLM when the standing setting allows it.
- S1 preferences, routines and personal context require an explicit, revocable grant for each session. Show the egress decision to the user in that session.
- S2 stays local in P5. No P5 provider path exports S2.
- S3 secrets, tokens and keys never enter model context, whether inference is local or hosted.
- The first memory slice is one S1 preference with provenance. Keep it local; include it in hosted inference only under the session's S1 egress grant.

A provider interface or user prompt is not itself an egress authorization. Enforce these rules in deterministic routing and context assembly before provider invocation.

## Evaluation order

1. Implement the synthetic provider path and verify the provider-neutral interfaces.
2. Run MODEL-01 and SPEECH-01 using synthetic data and, only with opt-in, non-sensitive evaluation data.
3. Measure grounding, tool-call schema validity, prompt-injection resistance, refusal and uncertainty behavior, latency, cancellation and cost. Preserve raw redacted results and exact version/fixture hashes under `evidence/<gate>/<run-id>/`.
4. Apply the license and retention/region review from [the license register](../../toolchain/LICENSE-REGISTER.md).
5. Do not promote a real provider until the named gate passes and the owner approves that candidate and its terms.

Raw audio, real user conversations, secrets, screen content and private window titles must not enter source control, ordinary test fixtures or evidence.

## Candidate research leads (not verified and not selected)

Names below are taken from the owner-requested brief. Recheck exact model/package versions, primary release sources, supported languages, latency on target hardware, license, data handling, retention, regions and prices before any trial. Leaderboard/blog claims are not evidence.

| Modality | Local candidates | Hosted candidates | Current P5 treatment |
|---|---|---|---|
| ASR | NVIDIA Nemotron streaming, Parakeet TDT, Whisper large-v3/turbo | Deepgram Nova-3, AssemblyAI, Azure Speech | Local-only; hosted ASR is excluded while raw-audio-local policy stands |
| TTS | Kokoro-82M, Chatterbox, CosyVoice, Piper | Cartesia, Inworld, ElevenLabs | Prefer local evaluation; Piper and every model license require review |
| LLM | gpt-oss-20b, Gemma, Qwen | Anthropic, OpenAI, Google | Hosted S0/S1 only under the egress rules; optional local fallback only if hardware supports it |
| Embeddings | Not researched | Not researched | Not needed for one preference; defer to MEM-01 |

No embeddings are required for M1–M4 or the first preference record.

## Owner inputs still needed for M5

- Hardware/GPU tier. The brief's “about 12 GB VRAM” local-model threshold is an unverified planning estimate, not an acceptance requirement.
- Languages and pass thresholds. “English first, Hindi stretch” is a proposal only.
- Hosted evaluation budget and time box. “US$20/month and two weeks” is a proposal only.
- Specific model/provider candidates and their processing terms.
