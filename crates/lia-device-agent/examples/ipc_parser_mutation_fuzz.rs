//! Deterministic mutation smoke for the bounded IPC frame decoder and schemas.
//! This is reproducible parser stress, not coverage-guided fuzzing or IPC-01.
use lia_device_agent::ipc::{Envelope, read_frame};
use std::io::Cursor;

const ITERATIONS: usize = 50_000;
const MAX_MUTATED_PAYLOAD: usize = 2_048;

fn main() {
    let corpus = seed_corpus();
    let mut state = 0x8f31_5a27_d4c9_b601_u64;
    let mut accepted = 0_usize;
    let mut rejected = 0_usize;
    let mut largest = 0_usize;

    for iteration in 0..ITERATIONS {
        let seed = &corpus[next(&mut state) as usize % corpus.len()];
        let mut payload = seed.clone();
        let mutations = 1 + (next(&mut state) as usize % 8);
        for _ in 0..mutations {
            match next(&mut state) % 4 {
                0 if !payload.is_empty() => {
                    let index = next(&mut state) as usize % payload.len();
                    payload[index] ^= (next(&mut state) as u8) | 1;
                }
                1 if payload.len() < MAX_MUTATED_PAYLOAD => {
                    let index = next(&mut state) as usize % (payload.len() + 1);
                    payload.insert(index, next(&mut state) as u8);
                }
                2 if !payload.is_empty() => {
                    let index = next(&mut state) as usize % payload.len();
                    payload.remove(index);
                }
                _ => {
                    let length = next(&mut state) as usize % 4;
                    payload.truncate(length.min(payload.len()));
                }
            }
        }
        if iteration % 97 == 0 {
            payload = (0..(next(&mut state) as usize % 128))
                .map(|_| next(&mut state) as u8)
                .collect();
        }
        largest = largest.max(payload.len());
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(&payload);
        match read_frame(&mut Cursor::new(frame)) {
            Ok(envelope) => {
                envelope
                    .validate()
                    .expect("decoder returned an invalid envelope");
                accepted += 1;
            }
            Err(_) => rejected += 1,
        }
    }

    println!("seed=0x8f315a27d4c9b601");
    println!("iterations={ITERATIONS}");
    println!("accepted={accepted}");
    println!("rejected={rejected}");
    println!("largest_payload_bytes={largest}");
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn seed_corpus() -> Vec<Vec<u8>> {
    let mut hello = base();
    hello.method = "presentation.hello".into();
    hello.kind = "request".into();
    hello.session_id = None;
    hello.body = serde_json::json!({"protocol_version":"1"});

    let mut ready = base();
    ready.method = "presentation.ready".into();
    ready.kind = "request".into();
    ready.body = serde_json::json!({"ready":true});

    let mut event = base();
    event.method = "presentation.user_event".into();
    event.kind = "request".into();
    event.session_sequence = Some("0".into());
    event.body = serde_json::json!({"control_id":"panel.hide","event":"clicked"});

    let mut snapshot = base();
    snapshot.method = "presentation.state.replace".into();
    snapshot.body = serde_json::json!({
        "state_revision": 1,
        "character_id": "synthetic",
        "emotion": {"id":"neutral", "intensity":0.0},
        "pose_id": "idle",
        "speaking": false,
        "listening": false,
        "paused": false
    });

    vec![
        serde_json::to_vec(&hello).unwrap(),
        serde_json::to_vec(&ready).unwrap(),
        serde_json::to_vec(&event).unwrap(),
        serde_json::to_vec(&snapshot).unwrap(),
        b"{}".to_vec(),
        vec![0xff, 0xfe, 0xfd],
        b"{\"protocol_version\":\"1\",\"protocol_version\":\"1\"}".to_vec(),
    ]
}

fn base() -> Envelope {
    Envelope {
        protocol_version: "1".into(),
        message_id: "00000000-0000-4000-8000-000000000001".into(),
        session_id: Some("00000000-0000-4000-8000-000000000002".into()),
        session_sequence: None,
        request_id: None,
        correlation_id: None,
        kind: "event".into(),
        method: "presentation.health".into(),
        sent_at: "2026-11-17T00:00:00Z".into(),
        deadline_at: None,
        body: serde_json::json!({}),
    }
}
