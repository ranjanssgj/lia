//! Reproducible local accept-loop reconnect and connection-burst measurement.
//! This is a synthetic service check, not full IPC-01 or a desktop support claim.
use lia_device_agent::ipc::{Envelope, IpcRunControl, PrivateSocket, read_frame, write_frame};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{Arc, atomic::Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RECONNECTS: u64 = 128;
const ABUSIVE_CONNECTION_BURST: usize = 64;

fn main() -> io::Result<()> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("lia-ipc-stress-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let socket_path = directory.join("agent.sock");
    let mut endpoint = PrivateSocket::bind(&socket_path)?;
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = Arc::clone(&stop);
    let (event_sender, event_receiver) = mpsc::sync_channel(4);
    let now_us = 1_794_916_800_000_000_i64;
    let server = thread::spawn(move || {
        let _keep_receiver_alive = event_receiver;
        let mut revision = 0_u64;
        endpoint.run_accept_loop(
            unsafe { libc::geteuid() },
            std::process::id() as i32,
            |session_id| {
                revision += 1;
                Ok(Envelope {
                    protocol_version: "1".into(),
                    message_id: message_uuid(1_000_000 + revision),
                    session_id: Some(session_id.into()),
                    session_sequence: None,
                    request_id: None,
                    correlation_id: None,
                    kind: "event".into(),
                    method: "presentation.state.replace".into(),
                    sent_at: "2026-11-17T12:00:00Z".into(),
                    deadline_at: None,
                    body: serde_json::json!({
                        "state_revision": revision,
                        "character_id": "synthetic",
                        "emotion": {"id":"neutral", "intensity":0.0},
                        "pose_id": "idle",
                        "speaking": false,
                        "listening": false,
                        "paused": false
                    }),
                })
            },
            &event_sender,
            || now_us,
            IpcRunControl {
                io_timeout: Duration::from_millis(500),
                stop: &server_stop,
            },
        )
    });

    let fd_baseline = open_fd_count()?;
    let rss_start_kib = high_water_rss_kib()?;
    let mut observed_fd_high_water = fd_baseline;

    let mut burst = Vec::with_capacity(ABUSIVE_CONNECTION_BURST);
    for _ in 0..ABUSIVE_CONNECTION_BURST {
        burst.push(UnixStream::connect(&socket_path)?);
    }
    drop(burst);

    let mut handshake_latencies_ns = Vec::with_capacity(RECONNECTS as usize);
    for index in 0..RECONNECTS {
        let latency = connect_and_receive_snapshot(&socket_path, index)?;
        handshake_latencies_ns.push(latency.as_nanos() as u64);
        observed_fd_high_water = observed_fd_high_water.max(open_fd_count()?);
    }

    stop.store(true, Ordering::Release);
    server
        .join()
        .map_err(|_| io::Error::other("accept-loop thread panicked"))??;
    drop(socket_path);
    fs::remove_dir(&directory)?;

    handshake_latencies_ns.sort_unstable();
    println!("abusive_connection_burst={ABUSIVE_CONNECTION_BURST}");
    println!("successful_reconnects={RECONNECTS}");
    println!("queue_capacity=4");
    println!("open_fds_baseline={fd_baseline}");
    println!("open_fds_observed_high_water={observed_fd_high_water}");
    println!("rss_hwm_start_kib={rss_start_kib}");
    println!("rss_hwm_end_kib={}", high_water_rss_kib()?);
    println!(
        "handshake_p50_us={}",
        percentile(&handshake_latencies_ns, 50) / 1_000
    );
    println!(
        "handshake_p95_us={}",
        percentile(&handshake_latencies_ns, 95) / 1_000
    );
    println!(
        "handshake_max_us={}",
        handshake_latencies_ns.last().copied().unwrap_or(0) / 1_000
    );
    Ok(())
}

fn connect_and_receive_snapshot(path: &PathBuf, index: u64) -> io::Result<Duration> {
    let started = Instant::now();
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request_id = message_uuid(10_000 + 2 * index);
    let mut hello = base_message(index * 2 + 1, "request", "presentation.hello", None);
    hello.request_id = Some(request_id.clone());
    hello.body = serde_json::json!({"protocol_version":"1"});
    write_frame(&mut stream, &hello)?;
    let response = read_frame(&mut stream)?;
    let session_id = response.session_id.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "hello response omitted session_id",
        )
    })?;
    if response.request_id.as_deref() != Some(request_id.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "hello response correlation mismatch",
        ));
    }
    let mut ready = base_message(
        index * 2 + 2,
        "request",
        "presentation.ready",
        Some(session_id.clone()),
    );
    ready.body = serde_json::json!({"ready":true});
    write_frame(&mut stream, &ready)?;
    let snapshot = read_frame(&mut stream)?;
    if snapshot.method != "presentation.state.replace"
        || snapshot.session_id.as_deref() != Some(session_id.as_str())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid session snapshot",
        ));
    }
    Ok(started.elapsed())
}

fn base_message(index: u64, kind: &str, method: &str, session_id: Option<String>) -> Envelope {
    Envelope {
        protocol_version: "1".into(),
        message_id: message_uuid(index),
        session_id,
        session_sequence: None,
        request_id: None,
        correlation_id: None,
        kind: kind.into(),
        method: method.into(),
        sent_at: "2026-11-17T12:00:00Z".into(),
        deadline_at: None,
        body: serde_json::json!({}),
    }
}

fn message_uuid(index: u64) -> String {
    format!("00000000-0000-4000-8000-{index:012x}")
}

fn open_fd_count() -> io::Result<usize> {
    let count = fs::read_dir("/proc/self/fd")?.count();
    Ok(count.saturating_sub(1)) // Exclude read_dir's own descriptor.
}

fn high_water_rss_kib() -> io::Result<u64> {
    let status = fs::read_to_string("/proc/self/status")?;
    let line = status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "VmHWM missing from proc status"))?;
    line.split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid VmHWM line"))?
        .parse()
        .map_err(io::Error::other)
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    let index = values.len().saturating_sub(1) * percentile / 100;
    values.get(index).copied().unwrap_or(0)
}
