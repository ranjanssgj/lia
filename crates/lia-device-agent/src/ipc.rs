//! Linux presentation IPC primitives. This module validates transport messages only;
//! it does not authorize operations or invoke adapters.

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::{Duration, Instant};

pub const MAX_FRAME_BYTES: usize = 262_144;
const ALLOWED_METHODS: &[&str] = &[
    "presentation.hello",
    "presentation.ready",
    "presentation.user_event",
    "presentation.state.replace",
    "presentation.state.patch",
    "presentation.action.play",
    "presentation.audio.state",
    "presentation.health",
    "presentation.shutdown",
];

const MAX_CONTROL_ID_BYTES: usize = 128;
const MAX_SUBMITTED_TEXT_BYTES: usize = 8_192;
const MAX_PRESENTATION_ID_BYTES: usize = 128;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HelloBody {
    protocol_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadyBody {
    ready: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EmotionState {
    id: String,
    intensity: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PresentationSnapshot {
    state_revision: u64,
    character_id: String,
    emotion: EmotionState,
    pose_id: String,
    speaking: bool,
    listening: bool,
    paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gaze_target_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attention_cue: Option<String>,
}

fn validate_bounded_identifier(value: &str, label: &str) -> io::Result<()> {
    if value.is_empty()
        || value.len() > MAX_PRESENTATION_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return invalid(format!("{label} has invalid syntax or length"));
    }
    Ok(())
}

fn validate_method_body(method: &str, kind: &str, body: &Value) -> io::Result<()> {
    match (method, kind) {
        ("presentation.hello", "request" | "response") => {
            let parsed: HelloBody = serde_json::from_value(body.clone())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if parsed.protocol_version != "1" {
                return invalid("hello body protocol_version must be 1");
            }
        }
        ("presentation.ready", "request") => {
            let parsed: ReadyBody = serde_json::from_value(body.clone())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if !parsed.ready {
                return invalid("ready request must report ready=true");
            }
        }
        ("presentation.state.replace", "event") => {
            let parsed: PresentationSnapshot = serde_json::from_value(body.clone())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            if parsed.state_revision > 9_007_199_254_740_991 {
                return invalid("state_revision exceeds the interoperable JSON integer range");
            }
            validate_bounded_identifier(&parsed.character_id, "character_id")?;
            validate_bounded_identifier(&parsed.emotion.id, "emotion.id")?;
            validate_bounded_identifier(&parsed.pose_id, "pose_id")?;
            if !parsed.emotion.intensity.is_finite()
                || !(0.0..=1.0).contains(&parsed.emotion.intensity)
            {
                return invalid("emotion intensity must be finite and within 0..=1");
            }
            for (label, value) in [
                ("gaze_target_class", parsed.gaze_target_class.as_deref()),
                ("attention_cue", parsed.attention_cue.as_deref()),
            ] {
                if let Some(value) = value {
                    validate_bounded_identifier(value, label)?;
                }
            }
        }
        ("presentation.user_event", "request") => UserEventBody::validate(body)?,
        _ => {
            if !body.is_object() {
                return invalid("IPC body must be a JSON object");
            }
        }
    }
    Ok(())
}

fn validate_method_kind(method: &str, kind: &str) -> io::Result<()> {
    let allowed = match method {
        "presentation.hello" => matches!(kind, "request" | "response"),
        "presentation.ready" | "presentation.user_event" => kind == "request",
        "presentation.state.replace"
        | "presentation.state.patch"
        | "presentation.action.play"
        | "presentation.audio.state"
        | "presentation.shutdown" => kind == "event",
        "presentation.health" => matches!(kind, "request" | "response"),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        invalid("IPC method is not valid with this message kind")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserEventKind {
    Clicked,
    Dismissed,
    TextSubmitted,
    InterruptionRequested,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserEventBody {
    pub control_id: String,
    pub event: UserEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl UserEventBody {
    fn validate(value: &Value) -> io::Result<()> {
        let body: Self = serde_json::from_value(value.clone())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if body.control_id.is_empty()
            || body.control_id.len() > MAX_CONTROL_ID_BYTES
            || !body.control_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return invalid("user-event control_id has invalid syntax or length");
        }
        match (body.event, body.text) {
            (UserEventKind::TextSubmitted, Some(text))
                if text.len() <= MAX_SUBMITTED_TEXT_BYTES =>
            {
                Ok(())
            }
            (UserEventKind::TextSubmitted, _) => {
                invalid("text_submitted requires text within the size limit")
            }
            (_, None) => Ok(()),
            (_, Some(_)) => invalid("text is only allowed with text_submitted"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol_version: String,
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Canonical unsigned decimal string. Only user-event requests carry this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_sequence: Option<String>,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub kind: String,
    pub method: String,
    pub sent_at: String,
    pub deadline_at: Option<String>,
    pub body: Value,
}

impl Envelope {
    pub fn validate(&self) -> io::Result<()> {
        if self.protocol_version != "1" {
            return invalid("unsupported IPC protocol version");
        }
        if !is_uuid(&self.message_id) {
            return invalid("message_id must be a UUID");
        }
        if self.kind.is_empty() || self.kind.len() > 128 {
            return invalid("invalid kind");
        }
        if !matches!(self.kind.as_str(), "request" | "response" | "event") {
            return invalid("invalid IPC message kind");
        }
        if !ALLOWED_METHODS.contains(&self.method.as_str()) {
            return invalid("IPC method is not allowlisted");
        }
        validate_method_kind(&self.method, &self.kind)?;
        match (self.method.as_str(), self.kind.as_str()) {
            ("presentation.hello", "request") => {
                if self.session_id.is_some() || self.session_sequence.is_some() {
                    return invalid("hello request must not carry a session or sequence");
                }
            }
            ("presentation.hello", "response") => {
                if self.session_id.as_deref().is_none_or(|id| !is_uuid(id))
                    || self.session_sequence.is_some()
                {
                    return invalid("hello response must issue a session UUID without a sequence");
                }
            }
            ("presentation.hello", _) => return invalid("hello must be a request or response"),
            ("presentation.user_event", "request") => {
                if self.session_id.as_deref().is_none_or(|id| !is_uuid(id))
                    || self
                        .session_sequence
                        .as_deref()
                        .is_none_or(|sequence| parse_sequence(sequence).is_err())
                {
                    return invalid("user-event request requires a session UUID and sequence");
                }
            }
            ("presentation.user_event", _) => {
                return invalid("presentation.user_event must be a request");
            }
            _ => {
                if self.session_id.as_deref().is_none_or(|id| !is_uuid(id))
                    || self.session_sequence.is_some()
                {
                    return invalid("non-hello messages require a session UUID and no sequence");
                }
            }
        }
        validate_method_body(&self.method, &self.kind, &self.body)?;
        crate::time::parse_rfc3339_us(&self.sent_at)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        if let Some(deadline) = self.deadline_at.as_deref() {
            crate::time::parse_rfc3339_us(deadline)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        }
        for value in [self.request_id.as_deref(), self.correlation_id.as_deref()]
            .into_iter()
            .flatten()
        {
            if !is_uuid(value) {
                return invalid("invalid request/correlation identifier");
            }
        }
        Ok(())
    }

    /// Reject expired requests using the caller's trusted wall-clock sample.
    pub fn validate_deadline(&self, now_us: i64) -> io::Result<()> {
        if let Some(deadline) = self.deadline_at.as_deref() {
            let deadline_us = crate::time::parse_rfc3339_us(deadline)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            if now_us >= deadline_us {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "IPC message deadline has expired",
                ));
            }
        }
        Ok(())
    }
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn parse_sequence(value: &str) -> io::Result<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return invalid("session sequence must be a canonical unsigned decimal string");
    }
    value
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionPhase {
    AwaitHello,
    AwaitReady,
    Ready,
}

/// Per-connection protocol state. A new instance gets a fresh OS-random session ID;
/// it becomes active only after hello and ready, and disconnect discards its watermark.
#[derive(Debug)]
pub struct PresentationSession {
    session_id: String,
    last_user_sequence: Option<u64>,
    phase: SessionPhase,
}

/// Single-slot mailbox for replaceable presentation snapshots. Producers may
/// coalesce an unsent older snapshot, but cannot publish a stale revision. Keep
/// user events in a separate lossless bounded queue.
#[derive(Debug, Default)]
pub struct SnapshotMailbox<T> {
    latest: Option<(u64, T)>,
    highest_revision: Option<u64>,
}

pub struct IpcRunControl<'a> {
    pub io_timeout: Duration,
    pub stop: &'a AtomicBool,
}

impl<T> SnapshotMailbox<T> {
    pub fn publish(&mut self, revision: u64, snapshot: T) -> io::Result<()> {
        if self
            .highest_revision
            .is_some_and(|highest| revision <= highest)
        {
            return invalid("snapshot revision must strictly increase");
        }
        self.highest_revision = Some(revision);
        self.latest = Some((revision, snapshot));
        Ok(())
    }

    pub fn take(&mut self) -> Option<(u64, T)> {
        self.latest.take()
    }

    pub fn pending(&self) -> bool {
        self.latest.is_some()
    }
}

impl PresentationSession {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            session_id: random_uuid_v4()?,
            last_user_sequence: None,
            phase: SessionPhase::AwaitHello,
        })
    }

    #[cfg(test)]
    fn with_id(session_id: String) -> io::Result<Self> {
        if !is_uuid(&session_id) {
            return invalid("session ID must be a UUID");
        }
        Ok(Self {
            session_id,
            last_user_sequence: None,
            phase: SessionPhase::Ready,
        })
    }

    /// Accept the first hello request and produce its correlated session response.
    /// The connection stays inactive unless the request and response both validate.
    pub fn accept_hello(&mut self, request: &Envelope, now_us: i64) -> io::Result<Envelope> {
        request.validate()?;
        if request.method != "presentation.hello" || request.kind != "request" {
            return invalid("first IPC request must be presentation.hello");
        }
        if self.phase != SessionPhase::AwaitHello {
            return invalid("presentation hello has already been accepted");
        }
        request.validate_deadline(now_us)?;

        let response = Envelope {
            protocol_version: "1".into(),
            message_id: random_uuid_v4()?,
            session_id: Some(self.session_id.clone()),
            session_sequence: None,
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            kind: "response".into(),
            method: "presentation.hello".into(),
            sent_at: crate::time::format_rfc3339_us(now_us),
            deadline_at: None,
            body: serde_json::json!({"protocol_version":"1"}),
        };
        response.validate()?;
        self.phase = SessionPhase::AwaitReady;
        Ok(response)
    }

    /// Perform the hello/ready exchange on a peer-authenticated Unix stream. The
    /// caller must authenticate the peer before calling this method and discard the
    /// consumed session if any I/O or protocol step fails.
    pub fn negotiate(
        mut self,
        stream: &mut UnixStream,
        mut trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
    ) -> io::Result<Self> {
        let result = (|| {
            let deadline = Instant::now().checked_add(io_timeout).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "IPC timeout overflow")
            })?;

            let hello = read_frame_until(stream, deadline)?;
            let response = self.accept_hello(&hello, trusted_now_us())?;
            write_frame_until(stream, &response, deadline)?;

            let ready = read_frame_until(stream, deadline)?;
            self.accept_ready(&ready, trusted_now_us())?;
            Ok(self)
        })();
        if result.is_err() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        result
    }

    /// Accept the renderer's readiness report for this session. User events are not
    /// accepted until this transition, so the caller can publish its fresh snapshot.
    pub fn accept_ready(&mut self, request: &Envelope, now_us: i64) -> io::Result<()> {
        request.validate()?;
        if request.method != "presentation.ready" || request.kind != "request" {
            return invalid("expected a presentation.ready request");
        }
        if self.phase != SessionPhase::AwaitReady {
            return invalid("presentation.ready is only accepted once after hello");
        }
        request.validate_deadline(now_us)?;
        if request.session_id.as_deref() != Some(self.session_id.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "IPC session ID does not match this connection",
            ));
        }
        self.phase = SessionPhase::Ready;
        Ok(())
    }

    /// Validate and consume a user-event sequence before any handler can dispatch it.
    /// This provides replay ordering only; it does not authenticate renderer authority.
    pub fn accept_user_event(&mut self, envelope: &Envelope, now_us: i64) -> io::Result<()> {
        envelope.validate()?;
        if self.phase != SessionPhase::Ready {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "presentation.ready must complete before user events are accepted",
            ));
        }
        if envelope.method != "presentation.user_event" || envelope.kind != "request" {
            return invalid("only user-event requests use the client sequence");
        }
        envelope.validate_deadline(now_us)?;
        if envelope.session_id.as_deref() != Some(self.session_id.as_str()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "IPC session ID does not match this connection",
            ));
        }
        let sequence = parse_sequence(
            envelope
                .session_sequence
                .as_deref()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing sequence"))?,
        )?;
        match self.last_user_sequence {
            None if sequence != 0 => return invalid("first user-event sequence must be zero"),
            Some(last) if sequence <= last => {
                return invalid("user-event sequence did not strictly increase");
            }
            _ => {}
        }
        self.last_user_sequence = Some(sequence);
        Ok(())
    }

    /// Read one framed user event from an already-negotiated stream, validate its
    /// session, deadline and sequence, and return the untrusted intent for caller
    /// policy checks. This method never dispatches an action. Any malformed,
    /// expired or replayed frame closes the connection and leaves no reusable session.
    pub fn receive_user_event(
        &mut self,
        stream: &mut UnixStream,
        mut trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
    ) -> io::Result<Envelope> {
        let result = (|| {
            let deadline = Instant::now().checked_add(io_timeout).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "IPC timeout overflow")
            })?;
            let event = read_frame_until(stream, deadline)?;
            self.accept_user_event(&event, trusted_now_us())?;
            Ok(event)
        })();
        if result.is_err() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        result
    }

    /// Send a fresh derived-state replacement after readiness. State delivery is
    /// server-to-renderer only; this does not grant the renderer an action path.
    pub fn send_snapshot(
        &self,
        stream: &mut UnixStream,
        snapshot: &Envelope,
        io_timeout: Duration,
    ) -> io::Result<()> {
        if self.phase != SessionPhase::Ready
            || snapshot.method != "presentation.state.replace"
            || snapshot.kind != "event"
            || snapshot.session_id.as_deref() != Some(self.session_id.as_str())
            || snapshot.session_sequence.is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "snapshot must be a state.replace event for this ready session",
            ));
        }
        let result = (|| {
            snapshot.validate()?;
            let deadline = Instant::now().checked_add(io_timeout).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "IPC timeout overflow")
            })?;
            write_frame_until(stream, snapshot, deadline)
        })();
        if result.is_err() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        result
    }

    /// Consume and validate one user event, then put it in a caller-sized bounded
    /// queue. A full/disconnected queue closes this session so backpressure is
    /// explicit. The queued envelope remains untrusted and is never dispatched here.
    pub fn receive_user_event_into(
        &mut self,
        stream: &mut UnixStream,
        trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
        queue: &SyncSender<Envelope>,
    ) -> io::Result<()> {
        let event = self.receive_user_event(stream, trusted_now_us, io_timeout)?;
        let result = match queue.try_send(event) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "presentation event queue is full; session closed for backpressure",
            )),
            Err(TrySendError::Disconnected(_)) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "presentation event receiver is unavailable",
            )),
        };
        if result.is_err() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        result
    }
}

fn random_uuid_v4() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("");
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

fn invalid<T>(message: impl Into<String>) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, message.into()))
}

fn remaining_until(deadline: Instant) -> io::Result<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "IPC handshake deadline has expired",
        ));
    }
    Ok(remaining)
}

fn read_exact_until(
    stream: &mut UnixStream,
    mut buffer: &mut [u8],
    deadline: Instant,
) -> io::Result<()> {
    while !buffer.is_empty() {
        stream.set_read_timeout(Some(remaining_until(deadline)?))?;
        match stream.read(buffer) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(read) => buffer = &mut buffer[read..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "IPC handshake read deadline has expired",
                ));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn write_all_until(
    stream: &mut UnixStream,
    mut buffer: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    while !buffer.is_empty() {
        stream.set_write_timeout(Some(remaining_until(deadline)?))?;
        match stream.write(buffer) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            Ok(written) => buffer = &buffer[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "IPC handshake write deadline has expired",
                ));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn read_frame_until(stream: &mut UnixStream, deadline: Instant) -> io::Result<Envelope> {
    let mut header = [0_u8; 4];
    read_exact_until(stream, &mut header, deadline)?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return invalid("IPC frame length is zero or exceeds the maximum");
    }
    let mut payload = vec![0; length];
    read_exact_until(stream, &mut payload, deadline)?;
    let text = std::str::from_utf8(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let unique = UniqueValue::deserialize(&mut deserializer)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    deserializer
        .end()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let envelope: Envelope = serde_json::from_value(unique.into_json())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    envelope.validate()?;
    Ok(envelope)
}

fn write_frame_until(
    stream: &mut UnixStream,
    envelope: &Envelope,
    deadline: Instant,
) -> io::Result<()> {
    envelope.validate()?;
    let payload = serde_json::to_vec(envelope)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
        return invalid("IPC frame length is zero or exceeds the maximum");
    }
    let length = u32::try_from(payload.len())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_all_until(stream, &length.to_be_bytes(), deadline)?;
    write_all_until(stream, &payload, deadline)
}

/// Parse one 4-byte big-endian length-prefixed UTF-8 JSON envelope.
pub fn read_frame<R: Read>(reader: &mut R) -> io::Result<Envelope> {
    let mut header = [0_u8; 4];
    reader.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return invalid("IPC frame length is zero or exceeds the maximum");
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    let text =
        std::str::from_utf8(&payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let unique = UniqueValue::deserialize(&mut deserializer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    deserializer
        .end()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let value = unique.into_json();
    let envelope: Envelope =
        serde_json::from_value(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    envelope.validate()?;
    Ok(envelope)
}

/// Write one validated frame and flush it to the stream.
pub fn write_frame<W: Write>(writer: &mut W, envelope: &Envelope) -> io::Result<()> {
    envelope.validate()?;
    let payload =
        serde_json::to_vec(envelope).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
        return invalid("IPC frame length is zero or exceeds the maximum");
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "IPC frame length overflow"))?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()
}

/// Peer credentials returned by Linux SO_PEERCRED, never read from JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerCredentials {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

/// Owns the renderer child and binds socket authentication to that exact launched
/// process. `Command` executes a program directly; callers must not configure a
/// shell wrapper or an untrusted executable as the renderer.
pub struct RendererSupervisor {
    command: Command,
    child: Option<Child>,
    expected_uid: u32,
    expected_pid: i32,
    restart_count: u8,
    next_restart_at: Option<Instant>,
}

const MAX_RENDERER_RESTARTS: u8 = 3;

impl RendererSupervisor {
    pub fn launch(mut command: Command) -> io::Result<Self> {
        let expected_uid = unsafe { libc::geteuid() };
        let child = command.spawn()?;
        let expected_pid = i32::try_from(child.id())
            .map_err(|_| io::Error::other("renderer PID exceeds kernel credential range"))?;
        Ok(Self {
            command,
            child: Some(child),
            expected_uid,
            expected_pid,
            restart_count: 0,
            next_restart_at: None,
        })
    }

    pub fn expected_peer(&self) -> (u32, i32) {
        (self.expected_uid, self.expected_pid)
    }

    /// Reap and relaunch an exited renderer, updating the expected PID before the
    /// accept loop handles another connection. Returns true when a new child starts.
    pub fn restart_if_exited(&mut self) -> io::Result<bool> {
        let Some(child) = self.child.as_mut() else {
            return Err(io::Error::other("renderer supervisor has no child"));
        };
        if child.try_wait()?.is_none() {
            return Ok(false);
        }
        if self.restart_count >= MAX_RENDERER_RESTARTS {
            return Err(io::Error::other("renderer restart limit exceeded"));
        }
        let exponent = u32::from(self.restart_count.min(5));
        let delay = Duration::from_millis(50_u64.saturating_mul(1_u64 << exponent));
        let restart_at = *self
            .next_restart_at
            .get_or_insert_with(|| Instant::now() + delay);
        if Instant::now() < restart_at {
            return Ok(false);
        }
        let child = self.command.spawn()?;
        self.expected_pid = i32::try_from(child.id())
            .map_err(|_| io::Error::other("renderer PID exceeds kernel credential range"))?;
        self.child = Some(child);
        self.restart_count += 1;
        self.next_restart_at = None;
        Ok(true)
    }
}

impl Drop for RendererSupervisor {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn peer_credentials(stream: &UnixStream) -> io::Result<PeerCredentials> {
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `credentials` points to writable storage of the exact ucred size, and
    // getsockopt initializes it on success. `stream` keeps the descriptor alive.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if length as usize != std::mem::size_of::<libc::ucred>() {
        return invalid("kernel returned an unexpected peer credential size");
    }
    // SAFETY: successful getsockopt initialized the complete ucred structure above.
    let credentials = unsafe { credentials.assume_init() };
    Ok(PeerCredentials {
        pid: credentials.pid,
        uid: credentials.uid,
        gid: credentials.gid,
    })
}

pub fn require_peer_process(
    stream: &UnixStream,
    expected_uid: u32,
    expected_pid: i32,
) -> io::Result<PeerCredentials> {
    let peer = peer_credentials(stream)?;
    if expected_pid <= 0 || peer.uid != expected_uid || peer.pid != expected_pid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Unix socket peer credentials do not match the expected process",
        ));
    }
    Ok(peer)
}

/// Bind only inside an already-private, owner-controlled directory. Existing paths
/// are never removed or replaced. Drop unlinks only the socket inode this instance made.
#[derive(Debug)]
pub struct PrivateSocket {
    listener: UnixListener,
    path: PathBuf,
    _parent_directory: File,
    device: u64,
    inode: u64,
}

impl PrivateSocket {
    /// Bind through a pinned directory FD so renaming/replacing the parent cannot
    /// redirect creation or cleanup. The parent must remain at its original path
    /// through the bind and permission checks; Linux procfs must be mounted.
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::bind_with_hooks(path, || {}, || {})
    }

    fn bind_with_hooks(
        path: impl AsRef<Path>,
        after_parent_validation: impl FnOnce(),
        after_endpoint_bind: impl FnOnce(),
    ) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let name = path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "socket path must have a filename",
            )
        })?;
        let parent_lstat = std::fs::symlink_metadata(&parent)?;
        if parent_lstat.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socket directory must not be a symlink",
            ));
        }
        let parent_directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&parent)?;
        let metadata = parent_directory.metadata()?;
        if !metadata.is_dir()
            || metadata.dev() != parent_lstat.dev()
            || metadata.ino() != parent_lstat.ino()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o777 != 0o700
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socket directory must be a stable, owner-controlled directory with mode 0700",
            ));
        }
        after_parent_validation();
        let anchored_path = Path::new("/proc/self/fd")
            .join(parent_directory.as_raw_fd().to_string())
            .join(name);
        match std::fs::symlink_metadata(&anchored_path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "refusing to replace an existing socket path",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&anchored_path)?;
        let metadata = std::fs::symlink_metadata(&anchored_path)?;
        let endpoint = Self {
            listener,
            path: anchored_path.clone(),
            _parent_directory: parent_directory,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "bound endpoint identity changed unexpectedly",
            ));
        }
        ensure_parent_path_matches(&parent, &endpoint._parent_directory)?;
        after_endpoint_bind();
        set_socket_mode_nofollow(&endpoint._parent_directory, name)?;
        let after = std::fs::symlink_metadata(&anchored_path)?;
        if !after.file_type().is_socket()
            || after.uid() != metadata.uid()
            || after.dev() != metadata.dev()
            || after.ino() != metadata.ino()
            || after.permissions().mode() & 0o777 != 0o600
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socket path changed while applying endpoint permissions",
            ));
        }
        ensure_parent_path_matches(&parent, &endpoint._parent_directory)?;
        Ok(endpoint)
    }

    pub fn listener(&self) -> &UnixListener {
        &self.listener
    }

    /// Accept a connection only after checking kernel credentials, before a caller
    /// can read or dispatch any framed message from that stream.
    pub fn accept_authenticated(
        &self,
        expected_uid: u32,
        expected_pid: i32,
    ) -> io::Result<(UnixStream, PeerCredentials)> {
        let (stream, _) = self.listener.accept()?;
        let peer = require_peer_process(&stream, expected_uid, expected_pid)?;
        Ok((stream, peer))
    }

    /// Accept a UID-authenticated peer, then complete a fresh hello/ready exchange.
    /// Each call creates a new session; callers must drop the stream/session together.
    pub fn accept_session(
        &self,
        expected_uid: u32,
        expected_pid: i32,
        mut trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
    ) -> io::Result<(UnixStream, PeerCredentials, PresentationSession)> {
        let (mut stream, peer) = self.accept_authenticated(expected_uid, expected_pid)?;
        let session = match PresentationSession::new() {
            Ok(session) => session,
            Err(error) => {
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return Err(error);
            }
        };
        let session = session.negotiate(&mut stream, &mut trusted_now_us, io_timeout)?;
        Ok((stream, peer, session))
    }

    /// Serve sequential connections from one supervised renderer process. Each
    /// successful hello/ready receives a fresh snapshot; disconnects and invalid
    /// sessions return to accept. Valid user events enter only the bounded intent
    /// queue and are never authorized or dispatched by this transport loop.
    pub fn run_accept_loop<F>(
        &mut self,
        expected_uid: u32,
        expected_pid: i32,
        snapshot_for_session: F,
        queue: &SyncSender<Envelope>,
        trusted_now_us: impl FnMut() -> i64,
        control: IpcRunControl<'_>,
    ) -> io::Result<()>
    where
        F: FnMut(&str) -> io::Result<Envelope>,
    {
        self.run_accept_loop_using(
            || Ok((expected_uid, expected_pid)),
            snapshot_for_session,
            queue,
            trusted_now_us,
            control.io_timeout,
            control.stop,
        )
    }

    /// Serve connections using the live PID from a renderer supervisor. If the
    /// child exits, the supervisor relaunches it and the new PID is required for
    /// subsequent accepted connections.
    pub fn run_supervised_accept_loop<F>(
        &mut self,
        supervisor: &mut RendererSupervisor,
        snapshot_for_session: F,
        queue: &SyncSender<Envelope>,
        trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
        stop: &AtomicBool,
    ) -> io::Result<()>
    where
        F: FnMut(&str) -> io::Result<Envelope>,
    {
        self.run_accept_loop_using(
            || {
                supervisor.restart_if_exited()?;
                Ok(supervisor.expected_peer())
            },
            snapshot_for_session,
            queue,
            trusted_now_us,
            io_timeout,
            stop,
        )
    }

    fn run_accept_loop_using<F, P>(
        &mut self,
        mut peer_binding: P,
        mut snapshot_for_session: F,
        queue: &SyncSender<Envelope>,
        mut trusted_now_us: impl FnMut() -> i64,
        io_timeout: Duration,
        stop: &AtomicBool,
    ) -> io::Result<()>
    where
        F: FnMut(&str) -> io::Result<Envelope>,
        P: FnMut() -> io::Result<(u32, i32)>,
    {
        self.listener.set_nonblocking(true)?;
        let result = (|| {
            while !stop.load(Ordering::Acquire) {
                let (expected_uid, expected_pid) = peer_binding()?;
                let (mut stream, _) = match self.accept_authenticated(expected_uid, expected_pid) {
                    Ok(accepted) => accepted,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::PermissionDenied => continue,
                    Err(error) => return Err(error),
                };

                let session = PresentationSession::new()?;
                let mut session =
                    match session.negotiate(&mut stream, &mut trusted_now_us, io_timeout) {
                        Ok(session) => session,
                        Err(_) => continue,
                    };
                let snapshot = snapshot_for_session(&session.session_id)?;
                if session
                    .send_snapshot(&mut stream, &snapshot, io_timeout)
                    .is_err()
                {
                    continue;
                }
                while !stop.load(Ordering::Acquire) {
                    match session.receive_user_event_into(
                        &mut stream,
                        &mut trusted_now_us,
                        io_timeout,
                        queue,
                    ) {
                        Ok(()) => {}
                        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                            return Err(error);
                        }
                        Err(_) => break,
                    }
                }
            }
            Ok(())
        })();
        let restore = self.listener.set_nonblocking(false);
        result.and(restore)
    }
}

fn set_socket_mode_nofollow(parent_directory: &File, name: &std::ffi::OsStr) -> io::Result<()> {
    let name = std::ffi::CString::new(name.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "socket name contains NUL"))?;
    // SAFETY: the directory FD is live; `name` is NUL-terminated and relative to
    // that FD; fchmodat2 with AT_SYMLINK_NOFOLLOW never follows a raced symlink.
    let result = unsafe {
        libc::syscall(
            libc::SYS_fchmodat2,
            parent_directory.as_raw_fd(),
            name.as_ptr(),
            0o600,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn ensure_parent_path_matches(parent: &Path, pinned_directory: &File) -> io::Result<()> {
    let pinned = pinned_directory.metadata()?;
    let current = std::fs::symlink_metadata(parent)?;
    if current.file_type().is_symlink()
        || !current.is_dir()
        || current.dev() != pinned.dev()
        || current.ino() != pinned.ino()
        || current.uid() != unsafe { libc::geteuid() }
        || current.permissions().mode() & 0o777 != 0o700
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "socket directory changed while binding the endpoint",
        ));
    }
    Ok(())
}

impl Drop for PrivateSocket {
    fn drop(&mut self) {
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path)
            && metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug)]
enum UniqueValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl UniqueValue {
    fn into_json(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::Array(values) => Value::Array(values.into_iter().map(Self::into_json).collect()),
            Self::Object(values) => {
                let mut map = Map::new();
                for (key, value) in values {
                    map.insert(key, value.into_json());
                }
                Value::Object(map)
            }
        }
    }
}

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue::Null)
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue::Null)
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue::Bool(value))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue::Number(Number::from(value)))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue::Number(Number::from(value)))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(UniqueValue::Number)
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue::String(value.to_owned()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue::String(value))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue::Array(values))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = BTreeSet::new();
                let mut values = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !keys.insert(key.clone()) {
                        return Err(de::Error::custom("duplicate JSON object key"));
                    }
                    values.push((key, map.next_value()?));
                }
                Ok(UniqueValue::Object(values))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn envelope() -> Envelope {
        Envelope {
            protocol_version: "1".into(),
            message_id: "00000000-0000-4000-8000-000000000001".into(),
            session_id: None,
            session_sequence: None,
            request_id: Some("00000000-0000-4000-8000-000000000002".into()),
            correlation_id: None,
            kind: "request".into(),
            method: "presentation.hello".into(),
            sent_at: "2026-11-15T00:00:00Z".into(),
            deadline_at: None,
            body: serde_json::json!({"protocol_version":"1"}),
        }
    }

    fn user_event(session_id: &str, sequence: &str) -> Envelope {
        let mut event = envelope();
        event.method = "presentation.user_event".into();
        event.session_id = Some(session_id.into());
        event.session_sequence = Some(sequence.into());
        event.body = serde_json::json!({"control_id":"control.primary","event":"clicked"});
        event
    }

    fn ready_request(session_id: &str) -> Envelope {
        let mut request = envelope();
        request.method = "presentation.ready".into();
        request.session_id = Some(session_id.into());
        request.body = serde_json::json!({"ready":true});
        request
    }

    fn snapshot_body(state_revision: u64) -> Value {
        serde_json::json!({
            "state_revision": state_revision,
            "character_id": "synthetic",
            "emotion": {"id":"neutral", "intensity":0.0},
            "pose_id": "idle",
            "speaking": false,
            "listening": false,
            "paused": false
        })
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut bytes = (payload.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(payload);
        bytes
    }

    fn encoded_frame(envelope: &Envelope) -> Vec<u8> {
        frame(&serde_json::to_vec(envelope).unwrap())
    }

    #[test]
    fn snapshot_mailbox_is_single_slot_coalesces_and_rejects_stale_revision() {
        let mut mailbox = SnapshotMailbox::default();
        mailbox.publish(4, "older").unwrap();
        mailbox.publish(5, "newer").unwrap();
        assert!(mailbox.pending());
        assert_eq!(mailbox.take(), Some((5, "newer")));
        assert!(!mailbox.pending());
        assert_eq!(
            mailbox.publish(5, "duplicate").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            mailbox.publish(3, "stale").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn handshake_and_snapshot_bodies_reject_unknown_or_invalid_fields() {
        let mut hello = envelope();
        hello.body = serde_json::json!({"protocol_version":"1", "claimed_uid":1000});
        assert_eq!(
            hello.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut ready = ready_request("00000000-0000-4000-8000-000000000004");
        ready.body = serde_json::json!({"ready":false});
        assert_eq!(
            ready.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut snapshot = envelope();
        snapshot.kind = "event".into();
        snapshot.method = "presentation.state.replace".into();
        snapshot.session_id = Some("00000000-0000-4000-8000-000000000004".into());
        snapshot.body = snapshot_body(1);
        snapshot.validate().unwrap();

        let mut forged = snapshot.clone();
        forged.body["operation_succeeded"] = Value::Bool(true);
        assert_eq!(
            forged.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut invalid_intensity = snapshot;
        invalid_intensity.body["emotion"]["intensity"] = Value::from(1.5);
        assert_eq!(
            invalid_intensity.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn method_direction_mismatches_are_rejected() {
        let mut renderer_forgery = envelope();
        renderer_forgery.method = "presentation.state.replace".into();
        assert_eq!(
            renderer_forgery.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut ready_reply = envelope();
        ready_reply.method = "presentation.ready".into();
        ready_reply.kind = "response".into();
        ready_reply.session_id = Some("00000000-0000-4000-8000-000000000004".into());
        ready_reply.body = serde_json::json!({"ready":true});
        assert_eq!(
            ready_reply.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn ready_session_delivers_bound_snapshot() {
        let id = "00000000-0000-4000-8000-000000000003";
        let session = PresentationSession::with_id(id.into()).unwrap();
        let (mut server, mut client) = UnixStream::pair().unwrap();
        let mut snapshot = envelope();
        snapshot.kind = "event".into();
        snapshot.method = "presentation.state.replace".into();
        snapshot.session_id = Some(id.into());
        snapshot.body = snapshot_body(9);
        session
            .send_snapshot(&mut server, &snapshot, Duration::from_secs(1))
            .unwrap();
        assert_eq!(read_frame(&mut client).unwrap(), snapshot);

        let mut wrong_session = snapshot.clone();
        wrong_session.session_id = Some("00000000-0000-4000-8000-000000000004".into());
        assert_eq!(
            session
                .send_snapshot(&mut server, &wrong_session, Duration::from_secs(1))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );

        let (mut server, client) = UnixStream::pair().unwrap();
        client.shutdown(std::net::Shutdown::Both).unwrap();
        let disconnected = session
            .send_snapshot(&mut server, &snapshot, Duration::from_secs(1))
            .unwrap_err();
        assert!(matches!(
            disconnected.kind(),
            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
        ));
    }

    #[test]
    fn full_bounded_user_event_queue_closes_session_without_dispatch() {
        let id = "00000000-0000-4000-8000-000000000003";
        let mut session = PresentationSession::with_id(id.into()).unwrap();
        let (mut client, mut server) = UnixStream::pair().unwrap();
        write_frame(&mut client, &user_event(id, "0")).unwrap();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let queued_before = user_event(id, "99");
        sender.send(queued_before.clone()).unwrap();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        assert_eq!(
            session
                .receive_user_event_into(&mut server, || now_us, Duration::from_secs(1), &sender,)
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(receiver.try_recv().unwrap(), queued_before);
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn frame_roundtrip_and_allowlist_validation() {
        let original = envelope();
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &original).unwrap();
        assert_eq!(read_frame(&mut Cursor::new(bytes)).unwrap(), original);
        let mut forbidden = original;
        forbidden.method = "system.execute".into();
        assert_eq!(
            write_frame(&mut Vec::new(), &forbidden).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut invalid_id = envelope();
        invalid_id.message_id = "not-a-uuid".into();
        assert_eq!(
            write_frame(&mut Vec::new(), &invalid_id)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let mut expired = envelope();
        expired.deadline_at = Some("2026-11-15T00:00:01Z".into());
        assert_eq!(
            expired
                .validate_deadline(crate::time::parse_rfc3339_us("2026-11-15T00:00:01Z").unwrap())
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn presentation_session_rejects_replays_reordering_and_cross_session_frames() {
        let id = "00000000-0000-4000-8000-000000000003";
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let mut session = PresentationSession::with_id(id.into()).unwrap();
        session
            .accept_user_event(&user_event(id, "0"), now_us)
            .unwrap();
        assert_eq!(
            session
                .accept_user_event(&user_event(id, "0"), now_us)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        session
            .accept_user_event(&user_event(id, "2"), now_us)
            .unwrap();
        assert_eq!(
            session
                .accept_user_event(&user_event(id, "1"), now_us)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            session
                .accept_user_event(
                    &user_event("00000000-0000-4000-8000-000000000004", "3"),
                    now_us
                )
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            session
                .accept_user_event(&user_event(id, "03"), now_us)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn expired_user_event_does_not_consume_sequence() {
        let id = "00000000-0000-4000-8000-000000000003";
        let before_deadline = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let at_deadline = crate::time::parse_rfc3339_us("2026-11-15T00:00:01Z").unwrap();
        let mut session = PresentationSession::with_id(id.into()).unwrap();
        let mut event = user_event(id, "0");
        event.deadline_at = Some("2026-11-15T00:00:01Z".into());

        assert_eq!(
            session
                .accept_user_event(&event, at_deadline)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        session.accept_user_event(&event, before_deadline).unwrap();
    }

    #[test]
    fn hello_issues_random_session_once_and_requires_ready_before_events() {
        let mut first = PresentationSession::new().unwrap();
        let second = PresentationSession::new().unwrap();
        assert!(is_uuid(&first.session_id));
        assert_ne!(first.session_id, second.session_id);
        assert_eq!(first.session_id.as_bytes()[14], b'4');
        let request = envelope();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let hello_response = first.accept_hello(&request, now_us).unwrap();
        hello_response.validate().unwrap();
        assert_eq!(hello_response.kind, "response");
        assert_eq!(hello_response.method, "presentation.hello");
        assert_eq!(
            hello_response.session_id.as_deref(),
            Some(first.session_id.as_str())
        );
        assert_eq!(
            hello_response.request_id.as_deref(),
            request.request_id.as_deref()
        );
        assert_eq!(hello_response.correlation_id, request.correlation_id);
        assert!(
            first
                .accept_hello(&request, now_us)
                .unwrap_err()
                .to_string()
                .contains("already been accepted")
        );
    }

    #[test]
    fn pending_or_expired_handshake_cannot_accept_user_event() {
        let mut session = PresentationSession::new().unwrap();
        let before_deadline = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let pending_event = user_event(&session.session_id, "0");
        assert_eq!(
            session
                .accept_user_event(&pending_event, before_deadline)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );

        let mut request = envelope();
        request.deadline_at = Some("2026-11-15T00:00:01Z".into());
        assert_eq!(
            session
                .accept_hello(
                    &request,
                    crate::time::parse_rfc3339_us("2026-11-15T00:00:01Z").unwrap()
                )
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(session.phase, SessionPhase::AwaitHello);
        session.accept_hello(&envelope(), before_deadline).unwrap();
        let event_before_ready = user_event(&session.session_id, "0");
        assert_eq!(
            session
                .accept_user_event(&event_before_ready, before_deadline)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );

        assert_eq!(
            session
                .accept_ready(
                    &ready_request("00000000-0000-4000-8000-000000000004"),
                    before_deadline
                )
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );

        let mut expired_ready = ready_request(&session.session_id);
        expired_ready.deadline_at = Some("2026-11-15T00:00:01Z".into());
        let at_deadline = crate::time::parse_rfc3339_us("2026-11-15T00:00:01Z").unwrap();
        assert_eq!(
            session
                .accept_ready(&expired_ready, at_deadline)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(session.phase, SessionPhase::AwaitReady);
        session
            .accept_ready(&ready_request(&session.session_id), before_deadline)
            .unwrap();
        assert!(
            session
                .accept_ready(&ready_request(&session.session_id), before_deadline)
                .is_err()
        );
        let active_event = user_event(&session.session_id, "0");
        session
            .accept_user_event(&active_event, before_deadline)
            .unwrap();
    }

    #[test]
    fn unix_stream_negotiates_hello_and_ready_before_returning_session() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let server_thread = std::thread::spawn(move || {
            PresentationSession::new()
                .unwrap()
                .negotiate(&mut server, || now_us, Duration::from_secs(2))
                .unwrap()
        });

        let hello = envelope();
        write_frame(&mut client, &hello).unwrap();
        let response = read_frame(&mut client).unwrap();
        assert_eq!(response.kind, "response");
        assert_eq!(response.request_id, hello.request_id);
        let ready = ready_request(response.session_id.as_deref().unwrap());
        write_frame(&mut client, &ready).unwrap();

        let session = server_thread.join().unwrap();
        assert_eq!(session.phase, SessionPhase::Ready);
        assert_eq!(
            session.session_id,
            response
                .session_id
                .expect("hello response has a session ID")
        );
    }

    #[test]
    fn negotiated_stream_reads_one_user_event_then_closes_on_replay() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let server_thread = std::thread::spawn(move || {
            let mut session = PresentationSession::new()
                .unwrap()
                .negotiate(&mut server, || now_us, Duration::from_secs(2))
                .unwrap();
            let accepted = session
                .receive_user_event(&mut server, || now_us, Duration::from_secs(2))
                .unwrap();
            let replay = session
                .receive_user_event(&mut server, || now_us, Duration::from_secs(2))
                .unwrap_err();
            (accepted.session_sequence, replay.kind())
        });

        let hello = envelope();
        write_frame(&mut client, &hello).unwrap();
        let response = read_frame(&mut client).unwrap();
        let session_id = response.session_id.unwrap();
        write_frame(&mut client, &ready_request(&session_id)).unwrap();
        write_frame(&mut client, &user_event(&session_id, "0")).unwrap();
        write_frame(&mut client, &user_event(&session_id, "0")).unwrap();

        assert_eq!(
            server_thread.join().unwrap(),
            (Some("0".into()), io::ErrorKind::InvalidData)
        );
        let mut response = [0_u8; 1];
        assert!(match client.read(&mut response) {
            Ok(0) => true,
            Err(error) => error.kind() == io::ErrorKind::ConnectionReset,
            Ok(_) => false,
        });
    }

    #[test]
    fn unix_stream_disconnect_mid_header_or_body_is_reported_as_eof() {
        for bytes in [vec![0, 0], vec![0, 0, 0, 3, b'{']] {
            let (mut client, mut server) = UnixStream::pair().unwrap();
            client.write_all(&bytes).unwrap();
            drop(client);
            assert_eq!(
                read_frame(&mut server).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
    }

    #[test]
    fn hello_read_timeout_shuts_down_the_unfinished_connection() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let server_thread = std::thread::spawn(move || {
            PresentationSession::new()
                .unwrap()
                .negotiate(&mut server, || now_us, Duration::from_millis(100))
                .unwrap_err()
        });

        assert!(matches!(
            server_thread.join().unwrap().kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        let mut response = [0_u8; 1];
        assert!(match client.read(&mut response) {
            Ok(0) => true,
            Err(error) => error.kind() == io::ErrorKind::ConnectionReset,
            Ok(_) => false,
        });
    }

    #[test]
    fn slow_drip_cannot_extend_the_absolute_handshake_deadline() {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let started = Instant::now();
        let server_thread = std::thread::spawn(move || {
            PresentationSession::new()
                .unwrap()
                .negotiate(&mut server, || now_us, Duration::from_millis(300))
                .unwrap_err()
        });

        for byte in [0_u8, 0, 0] {
            client.write_all(&[byte]).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
        let error = server_thread.join().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(450));
    }

    #[test]
    fn authenticated_listener_issues_fresh_session_and_rejects_stale_ready() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-reconnect-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        let server_path = path.clone();
        let server = std::thread::spawn(move || {
            let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
            let (stream, peer, session) = endpoint
                .accept_session(
                    unsafe { libc::geteuid() },
                    std::process::id() as i32,
                    || now_us,
                    Duration::from_secs(2),
                )
                .unwrap();
            assert_eq!(peer.uid, unsafe { libc::geteuid() });
            drop(stream);
            drop(session);

            let result = endpoint.accept_session(
                unsafe { libc::geteuid() },
                std::process::id() as i32,
                || now_us,
                Duration::from_secs(2),
            );
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
            drop(endpoint);
            assert!(!server_path.exists());
        });

        let mut first = UnixStream::connect(&path).unwrap();
        write_frame(&mut first, &envelope()).unwrap();
        let first_response = read_frame(&mut first).unwrap();
        let old_session_id = first_response.session_id.unwrap();
        write_frame(&mut first, &ready_request(&old_session_id)).unwrap();
        drop(first);

        let mut second = UnixStream::connect(&path).unwrap();
        write_frame(&mut second, &envelope()).unwrap();
        let second_response = read_frame(&mut second).unwrap();
        let new_session_id = second_response.session_id.unwrap();
        assert_ne!(new_session_id, old_session_id);
        write_frame(&mut second, &ready_request(&old_session_id)).unwrap();
        server.join().unwrap();
        drop(second);
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn accept_loop_serves_reconnect_with_fresh_session_and_snapshot() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "lia-ipc-accept-loop-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let mut endpoint = PrivateSocket::bind(&path).unwrap();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let thread_stop = std::sync::Arc::clone(&stop);
        let (events, _received_events) = std::sync::mpsc::sync_channel(4);
        let now_us = crate::time::parse_rfc3339_us("2026-11-15T00:00:00Z").unwrap();
        let server = std::thread::spawn(move || {
            let mut revision = 0_u64;
            endpoint.run_accept_loop(
                unsafe { libc::geteuid() },
                std::process::id() as i32,
                |session_id| {
                    revision += 1;
                    let mut snapshot = envelope();
                    snapshot.kind = "event".into();
                    snapshot.method = "presentation.state.replace".into();
                    snapshot.session_id = Some(session_id.into());
                    snapshot.body = snapshot_body(revision);
                    Ok(snapshot)
                },
                &events,
                || now_us,
                IpcRunControl {
                    io_timeout: Duration::from_secs(2),
                    stop: &thread_stop,
                },
            )
        });

        let mut session_ids = Vec::new();
        for _ in 0..2 {
            let mut client = UnixStream::connect(&path).unwrap();
            write_frame(&mut client, &envelope()).unwrap();
            let hello = read_frame(&mut client).unwrap();
            let session_id = hello.session_id.unwrap();
            write_frame(&mut client, &ready_request(&session_id)).unwrap();
            let snapshot = read_frame(&mut client).unwrap();
            assert_eq!(snapshot.method, "presentation.state.replace");
            assert_eq!(snapshot.session_id.as_deref(), Some(session_id.as_str()));
            session_ids.push(session_id);
        }
        assert_ne!(session_ids[0], session_ids[1]);
        stop.store(true, Ordering::Release);
        server.join().unwrap().unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn renderer_supervisor_binds_and_refreshes_launched_process_pid() {
        const RENDERER_ENV: &str = "LIA_IPC_TEST_RENDERER_PATH";
        if let Some(path) = std::env::var_os(RENDERER_ENV) {
            let mut client = UnixStream::connect(path).unwrap();
            write_frame(&mut client, &envelope()).unwrap();
            let hello = read_frame(&mut client).unwrap();
            let session_id = hello.session_id.unwrap();
            write_frame(&mut client, &ready_request(&session_id)).unwrap();
            let snapshot = read_frame(&mut client).unwrap();
            assert_eq!(snapshot.method, "presentation.state.replace");
            return;
        }

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-supervisor-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "ipc::tests::renderer_supervisor_binds_and_refreshes_launched_process_pid",
                "--nocapture",
            ])
            .env(RENDERER_ENV, &path);
        let mut supervisor = RendererSupervisor::launch(command).unwrap();
        let (uid, pid) = supervisor.expected_peer();
        let (mut stream, peer, session) = endpoint
            .accept_session(uid, pid, || 1_795_708_800_000_000, Duration::from_secs(3))
            .unwrap();
        assert_eq!(peer.uid, uid);
        assert_eq!(peer.pid, pid);
        let mut snapshot = envelope();
        snapshot.kind = "event".into();
        snapshot.method = "presentation.state.replace".into();
        snapshot.session_id = Some(session.session_id.clone());
        snapshot.body = snapshot_body(1);
        session
            .send_snapshot(&mut stream, &snapshot, Duration::from_secs(2))
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        while !supervisor.restart_if_exited().unwrap() {
            assert!(
                Instant::now() < deadline,
                "renderer child did not exit for restart"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let (restart_uid, restart_pid) = supervisor.expected_peer();
        assert_eq!(restart_uid, uid);
        let (mut stream, peer, session) = endpoint
            .accept_session(
                restart_uid,
                restart_pid,
                || 1_795_708_800_000_000,
                Duration::from_secs(3),
            )
            .unwrap();
        assert_eq!(peer.pid, restart_pid);
        let mut snapshot = envelope();
        snapshot.kind = "event".into();
        snapshot.method = "presentation.state.replace".into();
        snapshot.session_id = Some(session.session_id.clone());
        snapshot.body = snapshot_body(2);
        session
            .send_snapshot(&mut stream, &snapshot, Duration::from_secs(2))
            .unwrap();
        drop(supervisor);
        drop(endpoint);
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    #[ignore = "requires user-namespace mappings for a distinct UID; run under unshare --map-auto -Ur"]
    fn namespace_rejects_foreign_owner_and_separate_uid() {
        const SOCKET_ENV: &str = "LIA_IPC_TEST_SEPARATE_UID_SOCKET";
        if let Some(path) = std::env::var_os(SOCKET_ENV) {
            let mut stream = UnixStream::connect(path).expect("test socket should be reachable");
            stream.write_all(b"probe").unwrap();
            println!("separate_uid_connected=true");
            return;
        }

        assert_eq!(
            unsafe { libc::geteuid() },
            0,
            "run this test in a mapped user namespace"
        );
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let foreign_directory = std::env::temp_dir().join(format!(
            "lia-ipc-foreign-owner-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&foreign_directory).unwrap();
        std::fs::set_permissions(&foreign_directory, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let foreign_cstring =
            std::ffi::CString::new(foreign_directory.as_os_str().as_encoded_bytes()).unwrap();
        // UID 65534 is mapped by `unshare --map-auto`; GID 0 remains the namespace owner.
        assert_eq!(
            unsafe { libc::chown(foreign_cstring.as_ptr(), 65_534, 0) },
            0
        );
        assert_eq!(
            PrivateSocket::bind(foreign_directory.join("agent.sock"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(unsafe { libc::chown(foreign_cstring.as_ptr(), 0, 0) }, 0);
        std::fs::remove_dir(&foreign_directory).unwrap();

        let private_directory = std::env::temp_dir().join(format!(
            "lia-ipc-separate-uid-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&private_directory).unwrap();
        std::fs::set_permissions(&private_directory, std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let socket_path = private_directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&socket_path).unwrap();
        // Relax only this disposable test endpoint so the distinct UID reaches
        // accept(); the production endpoint remains parent 0700/socket 0600.
        std::fs::set_permissions(&private_directory, std::fs::Permissions::from_mode(0o711))
            .unwrap();
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o666)).unwrap();
        let mut command = Command::new("setpriv");
        command
            .args(["--reuid=65534", "--regid=65534", "--clear-groups", "--"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "ipc::tests::namespace_rejects_foreign_owner_and_separate_uid",
                "--nocapture",
            ])
            .env(SOCKET_ENV, &socket_path);
        let child = command.spawn().unwrap();
        let expected_uid = unsafe { libc::geteuid() } as u32;
        let expected_pid = child.id() as i32;
        assert_eq!(
            endpoint
                .accept_authenticated(expected_uid, expected_pid)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "separate-UID client did not reach the socket: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        drop(endpoint);
        std::fs::remove_dir(private_directory).unwrap();
    }

    #[test]
    fn rejects_zero_oversized_truncated_utf8_and_malformed_frames() {
        assert_eq!(
            read_frame(&mut Cursor::new([0_u8, 0, 0]))
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert_eq!(
            read_frame(&mut Cursor::new(0_u32.to_be_bytes()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_frame(&mut Cursor::new(
                ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes()
            ))
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_frame(&mut Cursor::new(vec![0, 0, 0, 2, b'{']))
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert_eq!(
            read_frame(&mut Cursor::new(frame(&[0xff])))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_frame(&mut Cursor::new(frame(b"not-json")))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            read_frame(&mut Cursor::new(frame(b"{} {}")))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_invalid_version_kind_method_and_sequence_overflow() {
        let mut cases = Vec::new();
        let mut bad_version = envelope();
        bad_version.protocol_version = "2".into();
        cases.push(bad_version);
        let mut bad_kind = envelope();
        bad_kind.kind = "unknown".into();
        cases.push(bad_kind);
        let mut bad_method = envelope();
        bad_method.method = "system.execute".into();
        cases.push(bad_method);

        for invalid_envelope in cases {
            assert_eq!(
                read_frame(&mut Cursor::new(encoded_frame(&invalid_envelope)))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidData
            );
        }

        let overflow = user_event(
            "00000000-0000-4000-8000-000000000003",
            "18446744073709551616",
        );
        assert_eq!(
            read_frame(&mut Cursor::new(encoded_frame(&overflow)))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_duplicate_keys_and_unknown_envelope_fields() {
        let duplicate = br#"{"protocol_version":"1","message_id":"00000000-0000-4000-8000-000000000001","message_id":"00000000-0000-4000-8000-000000000001"}"#;
        assert_eq!(
            read_frame(&mut Cursor::new(frame(duplicate)))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let extra = br#"{"protocol_version":"1","message_id":"00000000-0000-4000-8000-000000000001","kind":"event","method":"presentation.hello","sent_at":"2026-11-15T00:00:00Z","body":{},"surprise":true}"#;
        assert_eq!(
            read_frame(&mut Cursor::new(frame(extra)))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn user_event_body_is_typed_bounded_and_cannot_claim_outcomes() {
        let session_id = "00000000-0000-4000-8000-000000000003";
        for body in [
            serde_json::json!({"control_id":"panel.hide","event":"clicked"}),
            serde_json::json!({"control_id":"chat.input","event":"text_submitted","text":"hello"}),
            serde_json::json!({"control_id":"chat.stop","event":"interruption_requested"}),
        ] {
            let mut event = user_event(session_id, "0");
            event.body = body;
            event.validate().unwrap();
        }

        let invalid_bodies = [
            serde_json::json!({"control_id":"panel.hide","event":"clicked","outcome":"success"}),
            serde_json::json!({"control_id":"panel.hide","event":"unknown"}),
            serde_json::json!({"control_id":"panel.hide","event":"clicked","text":"unexpected"}),
            serde_json::json!({"control_id":"chat.input","event":"text_submitted"}),
            serde_json::json!({"control_id":"","event":"clicked"}),
            serde_json::json!({"control_id":"chat.input","event":"text_submitted","text":"x".repeat(MAX_SUBMITTED_TEXT_BYTES + 1)}),
        ];
        for body in invalid_bodies {
            let mut event = user_event(session_id, "0");
            event.body = body;
            assert_eq!(
                event.validate().unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }

        let mut not_a_request = user_event(session_id, "0");
        not_a_request.kind = "event".into();
        assert_eq!(
            not_a_request.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn private_socket_checks_directory_mode_and_kernel_peer_credentials() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let _client = UnixStream::connect(&path).unwrap();
        let (server, peer) = endpoint
            .accept_authenticated(unsafe { libc::geteuid() }, std::process::id() as i32)
            .unwrap();
        assert_eq!(peer.pid, std::process::id() as i32);
        assert_eq!(
            require_peer_process(&server, peer.uid.wrapping_add(1), peer.pid)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            require_peer_process(&server, peer.uid, peer.pid + 1)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        drop(endpoint);
        assert!(!path.exists());
        let _ = std::fs::remove_dir(directory);
    }

    #[test]
    fn authenticated_accept_rejects_uid_and_pid_before_reading_untrusted_frame() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "lia-ipc-peer-reject-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        let expected_uid = unsafe { libc::geteuid() };
        let expected_pid = std::process::id() as i32;
        let server = std::thread::spawn(move || {
            for (uid, pid) in [
                (expected_uid.wrapping_add(1), expected_pid),
                (expected_uid, expected_pid + 1),
            ] {
                let error = endpoint
                    .accept_session(uid, pid, || 1_795_708_800_000_000, Duration::from_secs(1))
                    .unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            }
        });

        for _ in 0..2 {
            let mut client = UnixStream::connect(&path).unwrap();
            let _ = client.write_all(b"this is not a frame");
            let mut response = [0_u8; 1];
            assert!(match client.read(&mut response) {
                Ok(0) => true,
                Err(error) => error.kind() == io::ErrorKind::ConnectionReset,
                Ok(_) => false,
            });
        }
        server.join().unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn same_uid_sibling_process_is_rejected_before_frame_parsing() {
        const SOCKET_ENV: &str = "LIA_IPC_TEST_SOCKET_PATH";
        if let Some(path) = std::env::var_os(SOCKET_ENV) {
            let mut client = UnixStream::connect(path).unwrap();
            let _ = client.write_all(b"this is not a frame");
            let mut response = [0_u8; 1];
            assert!(match client.read(&mut response) {
                Ok(0) => true,
                Err(error) => error.kind() == io::ErrorKind::ConnectionReset,
                Ok(_) => false,
            });
            return;
        }

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-sibling-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        let executable = std::env::current_exe().unwrap();
        let child = Command::new(executable)
            .args([
                "--exact",
                "ipc::tests::same_uid_sibling_process_is_rejected_before_frame_parsing",
                "--nocapture",
            ])
            .env(SOCKET_ENV, &path)
            .spawn()
            .unwrap();

        let error = endpoint
            .accept_session(
                unsafe { libc::geteuid() },
                std::process::id() as i32,
                || 1_795_708_800_000_000,
                Duration::from_secs(2),
            )
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "sibling process test failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        drop(endpoint);
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn refuses_preexisting_socket_path_and_non_private_directory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-reject-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("agent.sock");
        assert_eq!(
            PrivateSocket::bind(&path).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(&path, b"keep").unwrap();
        assert_eq!(
            PrivateSocket::bind(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn refuses_symlink_parent_and_endpoint_without_touching_targets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("lia-ipc-symlink-{}-{nonce}", std::process::id()));
        let real_directory = root.join("real");
        std::fs::create_dir_all(&real_directory).unwrap();
        std::fs::set_permissions(&real_directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let symlink_directory = root.join("linked");
        std::os::unix::fs::symlink(&real_directory, &symlink_directory).unwrap();
        assert_eq!(
            PrivateSocket::bind(symlink_directory.join("agent.sock"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert!(!real_directory.join("agent.sock").exists());

        let target = real_directory.join("target");
        std::fs::write(&target, b"keep").unwrap();
        let endpoint = real_directory.join("agent.sock");
        std::os::unix::fs::symlink(&target, &endpoint).unwrap();
        assert_eq!(
            PrivateSocket::bind(&endpoint).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");

        std::fs::remove_file(endpoint).unwrap();
        std::fs::remove_file(target).unwrap();
        std::fs::remove_file(symlink_directory).unwrap();
        std::fs::remove_dir(real_directory).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn bind_rejects_parent_swap_race_and_cleans_pinned_directory() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lia-ipc-parent-race-{}-{nonce}",
            std::process::id()
        ));
        let parent = root.join("runtime");
        let moved_parent = root.join("runtime-moved");
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket_path = parent.join("agent.sock");
        let race_parent = parent.clone();
        let race_moved_parent = moved_parent.clone();
        let error = PrivateSocket::bind_with_hooks(
            &socket_path,
            move || {
                std::fs::rename(&race_parent, &race_moved_parent).unwrap();
                std::fs::create_dir(&race_parent).unwrap();
                std::fs::set_permissions(&race_parent, std::fs::Permissions::from_mode(0o700))
                    .unwrap();
            },
            || {},
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&moved_parent).unwrap().count(), 0);
        std::fs::remove_dir(parent).unwrap();
        std::fs::remove_dir(moved_parent).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn bind_never_chmods_raced_symlink_target() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "lia-ipc-endpoint-race-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket_path = directory.join("agent.sock");
        let target = directory.join("sentinel");
        std::fs::write(&target, b"preserve mode and contents").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        let race_path = socket_path.clone();
        let race_target = target.clone();
        let error = PrivateSocket::bind_with_hooks(
            &socket_path,
            || {},
            move || {
                std::fs::remove_file(&race_path).unwrap();
                std::os::unix::fs::symlink(&race_target, &race_path).unwrap();
            },
        )
        .unwrap_err();
        assert_ne!(error.kind(), io::ErrorKind::NotFound);
        assert!(
            std::fs::symlink_metadata(&socket_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"preserve mode and contents"
        );
        std::fs::remove_file(socket_path).unwrap();
        std::fs::remove_file(target).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn socket_drop_does_not_unlink_a_replacement_inode() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lia-ipc-inode-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.join("agent.sock");
        let endpoint = PrivateSocket::bind(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let replacement = UnixListener::bind(&path).unwrap();
        drop(endpoint);
        assert!(
            std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_socket()
        );
        drop(replacement);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
