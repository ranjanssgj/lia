//! Linux presentation IPC primitives. This module validates transport messages only;
//! it does not authorize operations or invoke adapters.

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol_version: String,
    pub message_id: String,
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
        for (name, value) in [
            ("message_id", self.message_id.as_str()),
            ("kind", self.kind.as_str()),
        ] {
            if value.is_empty() || value.len() > 128 {
                return invalid(format!("invalid {name}"));
            }
        }
        if !matches!(self.kind.as_str(), "request" | "response" | "event") {
            return invalid("invalid IPC message kind");
        }
        if !ALLOWED_METHODS.contains(&self.method.as_str()) {
            return invalid("IPC method is not allowlisted");
        }
        if !self.body.is_object() {
            return invalid("IPC body must be a JSON object");
        }
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
            if value.is_empty() || value.len() > 128 {
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

fn invalid<T>(message: impl Into<String>) -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::InvalidData, message.into()))
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

pub fn require_peer_uid(stream: &UnixStream, expected_uid: u32) -> io::Result<PeerCredentials> {
    let peer = peer_credentials(stream)?;
    if peer.uid != expected_uid || peer.pid <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Unix socket peer credentials do not match the expected user",
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
    device: u64,
    inode: u64,
}

impl PrivateSocket {
    pub fn bind(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let metadata = std::fs::symlink_metadata(parent)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "socket directory must be a non-symlink directory owned by this user with mode 0700",
            ));
        }
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "refusing to replace an existing socket path",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        let endpoint = Self {
            listener,
            path: path.clone(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "bound endpoint identity changed unexpectedly",
            ));
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let after = std::fs::symlink_metadata(&path)?;
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
        Ok(endpoint)
    }

    pub fn listener(&self) -> &UnixListener {
        &self.listener
    }
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn envelope() -> Envelope {
        Envelope {
            protocol_version: "1".into(),
            message_id: "message-1".into(),
            request_id: Some("request-1".into()),
            correlation_id: None,
            kind: "request".into(),
            method: "presentation.hello".into(),
            sent_at: "2026-11-15T00:00:00Z".into(),
            deadline_at: None,
            body: serde_json::json!({"renderer_build":"test"}),
        }
    }

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut bytes = (payload.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(payload);
        bytes
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
    fn rejects_zero_oversized_truncated_utf8_and_malformed_frames() {
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
    }

    #[test]
    fn rejects_duplicate_keys_and_unknown_envelope_fields() {
        let duplicate = br#"{"protocol_version":"1","message_id":"a","message_id":"b"}"#;
        assert_eq!(
            read_frame(&mut Cursor::new(frame(duplicate)))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let extra = br#"{"protocol_version":"1","message_id":"a","kind":"event","method":"presentation.hello","sent_at":"2026-11-15T00:00:00Z","body":{},"surprise":true}"#;
        assert_eq!(
            read_frame(&mut Cursor::new(frame(extra)))
                .unwrap_err()
                .kind(),
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
        let (server, _) = endpoint.listener().accept().unwrap();
        let peer = require_peer_uid(&server, unsafe { libc::geteuid() }).unwrap();
        assert_eq!(peer.pid, std::process::id() as i32);
        assert_eq!(
            require_peer_uid(&server, peer.uid.wrapping_add(1))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        drop(endpoint);
        assert!(!path.exists());
        let _ = std::fs::remove_dir(directory);
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
}
