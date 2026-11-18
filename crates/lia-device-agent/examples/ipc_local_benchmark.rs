//! Reproducible local UnixStream framing benchmark; not a compositor or full-service test.
use lia_device_agent::ipc::{Envelope, read_frame, write_frame};
use std::hint::black_box;
use std::io;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

const ITERATIONS: usize = 20_000;

fn main() -> io::Result<()> {
    let (mut client, mut server) = UnixStream::pair()?;
    let server_thread = thread::spawn(move || -> io::Result<()> {
        for _ in 0..ITERATIONS {
            let mut request = read_frame(&mut server)?;
            request.kind = "response".into();
            write_frame(&mut server, &request)?;
        }
        Ok(())
    });

    let mut samples = Vec::with_capacity(ITERATIONS);
    for index in 0..ITERATIONS {
        let mut request = envelope(index);
        let start = Instant::now();
        write_frame(&mut client, &request)?;
        request = read_frame(&mut client)?;
        black_box(request);
        samples.push(start.elapsed());
    }
    server_thread
        .join()
        .map_err(|_| io::Error::other("benchmark server thread panicked"))??;
    samples.sort_unstable();

    println!("iterations={ITERATIONS}");
    println!("round_trip_p50_us={}", percentile(&samples, 50).as_micros());
    println!("round_trip_p95_us={}", percentile(&samples, 95).as_micros());
    println!("round_trip_p99_us={}", percentile(&samples, 99).as_micros());
    println!(
        "round_trip_max_us={}",
        samples.last().copied().unwrap_or_default().as_micros()
    );
    if let Ok(status) = std::fs::read_to_string("/proc/self/status")
        && let Some(line) = status.lines().find(|line| line.starts_with("VmRSS:"))
    {
        println!("process_{line}");
    }
    if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
        println!("process_open_fd_count={}", entries.count());
    }
    Ok(())
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let index = samples
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1);
    samples[index.min(samples.len() - 1)]
}

fn envelope(index: usize) -> Envelope {
    Envelope {
        protocol_version: "1".into(),
        message_id: format!("00000000-0000-4000-8000-{index:012x}"),
        session_id: Some("00000000-0000-4000-8000-000000000001".into()),
        session_sequence: None,
        request_id: None,
        correlation_id: None,
        kind: "request".into(),
        method: "presentation.health".into(),
        sent_at: "2026-11-16T00:00:00Z".into(),
        deadline_at: None,
        body: serde_json::json!({"status":"ready"}),
    }
}
