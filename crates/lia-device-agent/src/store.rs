use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
    params_from_iter, types::Value,
};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        include_str!("../../../docs/p4/sql/sqlite/001-device-agent.sql"),
    ),
    (
        2,
        include_str!("../../../docs/p4/sql/sqlite/002-retention-tombstones.sql"),
    ),
    (
        3,
        include_str!("../../../docs/p4/sql/sqlite/003-consumption-ledger.sql"),
    ),
    (
        4,
        include_str!("../../../docs/p4/sql/sqlite/004-manifest-revision-json-safe.sql"),
    ),
    (
        5,
        include_str!("../../../docs/p4/sql/sqlite/005-sequence-zero-initial-use.sql"),
    ),
    (
        6,
        include_str!("../../../docs/p4/sql/sqlite/006-operation-outcomes-and-evidence.sql"),
    ),
    (
        7,
        include_str!("../../../docs/p4/sql/sqlite/007-integer-time-and-integrity-anchor.sql"),
    ),
    (
        8,
        include_str!("../../../docs/p4/sql/sqlite/008-integer-time-table-rebuild.sql"),
    ),
];

fn enable_wal_with_retry(conn: &Connection) -> Result<(), StoreError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match conn.pragma_update(None, "journal_mode", "WAL") {
            Ok(()) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(error, _))
                if matches!(
                    error.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn backfill_integer_times(tx: &Transaction<'_>) -> Result<(), StoreError> {
    let maps: &[(&str, &[(&str, &str)])] = &[
        (
            "local_operation_journal",
            &[
                ("received_at", "received_at_us"),
                ("expires_at", "expires_at_us"),
                ("updated_at", "updated_at_us"),
            ],
        ),
        (
            "local_operation_attempt",
            &[
                ("started_at", "started_at_us"),
                ("completed_at", "completed_at_us"),
            ],
        ),
        (
            "local_session_sequence",
            &[
                ("updated_at", "updated_at_us"),
                ("retain_until", "retain_until_us"),
            ],
        ),
        (
            "local_consumed_nonce",
            &[
                ("consumed_at", "consumed_at_us"),
                ("expires_at", "expires_at_us"),
            ],
        ),
        (
            "local_offline_lease",
            &[
                ("issued_at", "issued_at_us"),
                ("not_before", "not_before_us"),
                ("expires_at", "expires_at_us"),
                ("received_at", "received_at_us"),
                ("consumed_at", "consumed_at_us"),
            ],
        ),
        (
            "local_outbox",
            &[
                ("occurred_at", "occurred_at_us"),
                ("expires_at", "expires_at_us"),
            ],
        ),
        (
            "local_operation_evidence",
            &[("observed_at", "observed_at_us")],
        ),
    ];
    for (table, pairs) in maps {
        let select = format!(
            "SELECT rowid, {} FROM {table}",
            pairs
                .iter()
                .map(|(old, _)| *old)
                .collect::<Vec<_>>()
                .join(",")
        );
        let mut stmt = tx.prepare(&select)?;
        let mut rows = stmt.query([])?;
        let mut batch = Vec::new();
        while let Some(row) = rows.next()? {
            let rowid: i64 = row.get(0)?;
            let mut ints = Vec::with_capacity(pairs.len());
            for (idx, (old, _)) in pairs.iter().enumerate() {
                let value: Option<String> = row.get(idx + 1)?;
                let converted = value
                    .as_deref()
                    .map(crate::time::parse_rfc3339_us)
                    .transpose()
                    .map_err(|e| {
                        StoreError::Invalid(format!("cannot backfill {table}.{old}: {e}"))
                    })?;
                ints.push(converted.map_or(Value::Null, Value::Integer));
            }
            batch.push((rowid, ints));
        }
        drop(rows);
        drop(stmt);
        let set = pairs
            .iter()
            .enumerate()
            .map(|(i, (_, new))| format!("{new}=COALESCE({new},?{})", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let update = format!("UPDATE {table} SET {set} WHERE rowid=?{}", pairs.len() + 1);
        for (rowid, mut values) in batch {
            values.push(Value::Integer(rowid));
            tx.execute(&update, params_from_iter(values))?;
        }
    }
    Ok(())
}

/// Supplies the wall-clock sample used for signed-message expiry and the durable rollback
/// high-water mark. Production uses the host clock; tests inject deterministic samples.
pub trait TrustedClock: Send + Sync {
    fn now_us(&self) -> Result<i64, StoreError>;
}

#[derive(Debug, Default)]
pub struct SystemTrustedClock;
impl TrustedClock for SystemTrustedClock {
    fn now_us(&self) -> Result<i64, StoreError> {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| StoreError::Invalid("system clock is before Unix epoch".into()))?;
        i64::try_from(d.as_micros())
            .map_err(|_| StoreError::Invalid("system clock overflow".into()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Anchor {
    sequence: i64,
    time_us: i64,
}
static ANCHOR_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
struct TestClock;
#[cfg(test)]
impl TrustedClock for TestClock {
    fn now_us(&self) -> Result<i64, StoreError> {
        crate::time::parse_rfc3339_us("2026-11-15T00:03:00Z")
    }
}
#[cfg(test)]
struct FixedClock(i64);
#[cfg(test)]
impl TrustedClock for FixedClock {
    fn now_us(&self) -> Result<i64, StoreError> {
        Ok(self.0)
    }
}

fn anchor_path(database: &Path) -> PathBuf {
    let mut name = database.as_os_str().to_os_string();
    name.push(".lia-anchor");
    PathBuf::from(name)
}
#[cfg(test)]
fn clear_database_files(path: &Path) {
    for candidate in [
        path.to_path_buf(),
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
        anchor_path(path),
    ] {
        let _ = fs::remove_file(candidate);
    }
}
fn read_anchor(path: &Path) -> Result<Option<Anchor>, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(StoreError::Invalid(
                    "external anchor must be a regular non-symlink file".into(),
                ));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o077 != 0 {
                    return Err(StoreError::Invalid(
                        "external anchor permissions must exclude group and other access".into(),
                    ));
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(StoreError::Invalid(format!(
                "cannot inspect external anchor: {e}"
            )));
        }
    }
    let mut file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(StoreError::Invalid(format!(
                "cannot read external anchor: {e}"
            )));
        }
    };
    let mut data = String::new();
    file.read_to_string(&mut data)
        .map_err(|e| StoreError::Invalid(format!("invalid external anchor: {e}")))?;
    let mut parts = data.trim().split(' ');
    if parts.next() != Some("LIA-ANCHOR-1") {
        return Err(StoreError::Invalid(
            "external anchor format/version invalid".into(),
        ));
    }
    let sequence = parts
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| StoreError::Invalid("external anchor sequence invalid".into()))?;
    let time_us = parts
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| StoreError::Invalid("external anchor time invalid".into()))?;
    if parts.next().is_some() || sequence < 0 || time_us < 0 {
        return Err(StoreError::Invalid("external anchor fields invalid".into()));
    }
    Ok(Some(Anchor { sequence, time_us }))
}
fn write_anchor(path: &Path, anchor: Anchor) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let suffix = ANCHOR_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".tmp.{}.{}", std::process::id(), suffix));
    let temp = PathBuf::from(temp);
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)?
    };
    #[cfg(not(unix))]
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let content = format!("LIA-ANCHOR-1 {} {}\n", anchor.sequence, anchor.time_us);
    if let Err(e) = file
        .write_all(content.as_bytes())
        .and_then(|_| file.sync_all())
    {
        let _ = fs::remove_file(&temp);
        return Err(StoreError::Invalid(format!(
            "cannot persist external anchor: {e}"
        )));
    }
    drop(file);
    fs::rename(&temp, path)?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn outcome_key(value: ObservedOutcome) -> &'static str {
    match value {
        ObservedOutcome::VerifiedSuccess => "verified_success",
        ObservedOutcome::VerifiedFailure => "verified_failure",
        ObservedOutcome::Unknown => "unknown",
    }
}

#[derive(Debug)]
pub enum StoreError {
    Sql(rusqlite::Error),
    Invalid(String),
}
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Invalid(e.to_string())
    }
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}

#[derive(Clone, Debug)]
pub struct OperationClaims {
    pub operation_id: String,
    pub account_id: String,
    pub target_device_id: String,
    pub target_credential_generation: i64,
    pub capability_id: String,
    pub capability_schema_version: i64,
    pub resource_type: String,
    pub resource_id: String,
    pub resource_generation: String,
    pub expected_resource_revision: Option<String>,
    pub resource_observed_at: Option<String>,
    pub resource_owning_device_id: Option<String>,
    pub arguments_schema_uri: String,
    pub arguments_sha256: [u8; 32],
    pub grant_reference: String,
    pub policy_version: i64,
    pub revocation_epoch: i64,
    pub risk_tier: i64,
    pub purpose: String,
    pub originating_task_id: Option<String>,
    pub received_at: String,
    pub expires_at: String,
}
#[derive(Clone, Debug)]
pub struct CommandClaims {
    pub command_id: String,
    pub nonce: String,
    pub session_id: String,
    pub session_sequence: i64,
    pub not_before: String,
    pub expires_at: String,
    pub nonce_expires_at: String,
}
#[derive(Clone, Debug)]
pub struct LeaseClaims {
    pub lease_id: String,
    pub issuer_id: String,
    pub audience: String,
    pub nonce: String,
    pub signing_key_generation: i64,
    pub issued_at: String,
    pub not_before: String,
    pub expires_at: String,
}
#[derive(Clone, Debug)]
pub struct LeaseConsumption<'a> {
    pub account_id: &'a str,
    pub lease_id: &'a str,
    pub nonce: &'a str,
    pub attempt_id: &'a str,
    pub attempt_number: i64,
    pub worker_generation: i64,
    pub at: &'a str,
    pub nonce_expires_at: &'a str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservedOutcome {
    VerifiedSuccess,
    VerifiedFailure,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationOutcomeState {
    NotAttempted,
    Unknown,
    VerifiedSuccess,
    VerifiedFailure,
}
#[derive(Clone, Debug)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub evidence_kind: String,
    pub outcome: ObservedOutcome,
    pub sha256: [u8; 32],
    pub reference: String,
    pub sensitivity: i64,
    pub observed_at: String,
}
#[derive(Clone, Debug)]
pub struct AttemptCompletion {
    pub operation_id: String,
    pub attempt_id: String,
    pub event_id: String,
    pub correlation_id: String,
    pub occurred_at: String,
    pub outbox_expires_at: String,
    pub outcome: ObservedOutcome,
    pub failure_code: Option<String>,
    pub evidence: Option<EvidenceRecord>,
}
/// Constructed only after the replay marker and possible-effect journal are committed.
#[derive(Debug)]
pub struct EffectPermit {
    operation_id: String,
    attempt_id: String,
    mode: &'static str,
}
impl EffectPermit {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }
    pub fn mode(&self) -> &str {
        self.mode
    }
}

pub struct AgentStore {
    conn: Connection,
    anchor_path: Option<PathBuf>,
    clock: Arc<dyn TrustedClock>,
}
impl AgentStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        #[cfg(test)]
        let clock: Arc<dyn TrustedClock> = Arc::new(TestClock);
        #[cfg(not(test))]
        let clock: Arc<dyn TrustedClock> = Arc::new(SystemTrustedClock);
        Self::open_with_clock(path, clock)
    }
    pub fn open_with_clock(
        path: impl AsRef<Path>,
        clock: Arc<dyn TrustedClock>,
    ) -> Result<Self, StoreError> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Self::configure(conn, Some(anchor_path(path)), clock)
    }
    pub fn open_in_memory() -> Result<Self, StoreError> {
        #[cfg(test)]
        let clock: Arc<dyn TrustedClock> = Arc::new(TestClock);
        #[cfg(not(test))]
        let clock: Arc<dyn TrustedClock> = Arc::new(SystemTrustedClock);
        Self::open_in_memory_with_clock(clock)
    }
    pub fn open_in_memory_with_clock(clock: Arc<dyn TrustedClock>) -> Result<Self, StoreError> {
        Self::configure(Connection::open_in_memory()?, None, clock)
    }
    fn configure(
        mut conn: Connection,
        anchor_path: Option<PathBuf>,
        clock: Arc<dyn TrustedClock>,
    ) -> Result<Self, StoreError> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        enable_wal_with_retry(&conn)?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        if conn.query_row("PRAGMA synchronous", [], |r| r.get::<_, i64>(0))? != 2 {
            return Err(StoreError::Invalid(
                "SQLite synchronous=FULL is required".into(),
            ));
        }
        conn.execute_batch("CREATE TABLE IF NOT EXISTS lia_schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));")?;
        for (version, sql) in MIGRATIONS {
            // Lock before checking the version. Two processes can otherwise both see a
            // missing migration and race to apply the same DDL.
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let exists: Option<i64> = tx
                .query_row(
                    "SELECT version FROM lia_schema_migrations WHERE version=?1",
                    [version],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_none() {
                if *version == 8 {
                    tx.execute_batch("DROP TRIGGER IF EXISTS local_session_sequence_monotonic;")?;
                    backfill_integer_times(&tx)?;
                }
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO lia_schema_migrations(version) VALUES (?1)",
                    [version],
                )?;
            }
            tx.commit()?;
        }
        let mut store = Self {
            conn,
            anchor_path,
            clock,
        };
        store.initialize_or_validate_anchor()?;
        Ok(store)
    }
    fn initialize_or_validate_anchor(&mut self) -> Result<(), StoreError> {
        let Some(anchor_file) = self.anchor_path.as_deref() else {
            let now = self.clock.now_us()?;
            self.conn.execute("UPDATE local_trusted_clock_state SET last_trusted_time_us=MAX(last_trusted_time_us,?1) WHERE singleton=1", [now])?;
            return Ok(());
        };
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sequence: i64 = tx.query_row(
            "SELECT anchor_sequence FROM local_integrity_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        let last_time: i64 = tx.query_row(
            "SELECT last_trusted_time_us FROM local_trusted_clock_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        match read_anchor(anchor_file)? {
            Some(anchor) if anchor.sequence == sequence && anchor.time_us >= last_time => {
                let now = self.clock.now_us()?;
                if now < anchor.time_us {
                    return Err(StoreError::Invalid(
                        "trusted wall clock rollback detected by external high-water mark".into(),
                    ));
                }
                let high_water = now.max(anchor.time_us);
                if high_water > anchor.time_us {
                    write_anchor(
                        anchor_file,
                        Anchor {
                            sequence,
                            time_us: high_water,
                        },
                    )?;
                }
                if high_water > last_time {
                    tx.execute("UPDATE local_trusted_clock_state SET last_trusted_time_us=?1 WHERE singleton=1", [high_water])?;
                }
            }
            Some(_) => {
                return Err(StoreError::Invalid("SQLite integrity/time state does not match external anchor; fail-closed reconciliation required".into()));
            }
            None => {
                let history: i64 = tx.query_row("SELECT (SELECT COUNT(*) FROM local_consumed_nonce)+(SELECT COUNT(*) FROM local_operation_attempt)+(SELECT COUNT(*) FROM local_session_sequence WHERE has_accepted_sequence=1)", [], |r| r.get(0))?;
                if sequence != 0 || last_time != 0 || history != 0 {
                    return Err(StoreError::Invalid("external integrity anchor is missing for a non-empty replay journal; coordinator reconciliation is required".into()));
                }
                let now = self.clock.now_us()?;
                if now < last_time {
                    return Err(StoreError::Invalid("trusted clock moved backwards".into()));
                }
                write_anchor(
                    anchor_file,
                    Anchor {
                        sequence: 0,
                        time_us: now,
                    },
                )?;
                tx.execute("UPDATE local_trusted_clock_state SET last_trusted_time_us=?1 WHERE singleton=1", [now])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    fn advance_integrity_anchor(
        tx: &Transaction<'_>,
        path: Option<&Path>,
        now_us: i64,
    ) -> Result<(), StoreError> {
        let sequence: i64 = tx.query_row(
            "SELECT anchor_sequence FROM local_integrity_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        let last_time: i64 = tx.query_row(
            "SELECT last_trusted_time_us FROM local_trusted_clock_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        if now_us < last_time {
            return Err(StoreError::Invalid(
                "trusted wall clock rollback detected".into(),
            ));
        }
        let anchor = Anchor {
            sequence,
            time_us: last_time,
        };
        if let Some(path) = path {
            if read_anchor(path)? != Some(anchor) {
                return Err(StoreError::Invalid(
                    "external integrity/time anchor mismatch; refusing protected write".into(),
                ));
            }
            let next = Anchor {
                sequence: sequence
                    .checked_add(1)
                    .ok_or_else(|| StoreError::Invalid("integrity sequence exhausted".into()))?,
                time_us: now_us,
            };
            // Write/flush the external anchor first. If the SQLite commit is interrupted,
            // startup observes the mismatch and fails closed rather than replaying.
            write_anchor(path, next)?;
        }
        tx.execute(
            "UPDATE local_integrity_state SET anchor_sequence=anchor_sequence+1 WHERE singleton=1",
            [],
        )?;
        tx.execute(
            "UPDATE local_trusted_clock_state SET last_trusted_time_us=?1 WHERE singleton=1",
            [now_us],
        )?;
        Ok(())
    }
    pub fn pragma_i64(&self, name: &str) -> Result<i64, StoreError> {
        Ok(self
            .conn
            .query_row(&format!("PRAGMA {name}"), [], |r| r.get(0))?)
    }
    pub fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(MAX(version),0) FROM lia_schema_migrations",
            [],
            |r| r.get(0),
        )?)
    }
    /// Create a transactionally consistent SQLite backup that retains replay ledgers.
    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<(), StoreError> {
        self.conn.backup(
            rusqlite::MAIN_DB,
            destination,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        Ok(())
    }
    /// Restore a backup into this connection. The caller must prevent concurrent writers.
    pub fn restore_from(&mut self, source: impl AsRef<Path>) -> Result<(), StoreError> {
        self.conn.restore(
            rusqlite::MAIN_DB,
            source,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "synchronous", "FULL")?;
        self.initialize_or_validate_anchor()?;
        Ok(())
    }
    /// On startup after an interrupted process, quarantine all committed in-flight effects.
    /// It preserves the possible-effect marker and never returns the operation to accepted.
    pub fn reconcile_uncertain_operations(&mut self, _at: &str) -> Result<usize, StoreError> {
        let now_us = self.clock.now_us()?;
        let trusted_at = crate::time::format_rfc3339_us(now_us);
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count = tx.execute(
            "UPDATE local_operation_journal SET current_status='reconciliation_required',updated_at=?1,updated_at_us=?2 WHERE current_status='running' AND side_effect_may_have_occurred=1",
            params![trusted_at, now_us],
        )?;
        tx.execute(
            "UPDATE local_operation_attempt SET attempt_status='reconciliation_required' WHERE attempt_status='started' AND operation_id IN (SELECT operation_id FROM local_operation_journal WHERE current_status='reconciliation_required' AND side_effect_may_have_occurred=1)",
            [],
        )?;
        Self::advance_integrity_anchor(&tx, self.anchor_path.as_deref(), now_us)?;
        tx.commit()?;
        Ok(count)
    }
    /// Record an adapter's final observation, its redacted evidence reference, and the
    /// matching outbox event in one transaction. Verified states cannot be written without
    /// evidence because migration 006 enforces that invariant as well.
    pub fn finish_attempt(&mut self, c: &AttemptCompletion) -> Result<(), StoreError> {
        let occurred_us = crate::time::parse_rfc3339_us(&c.occurred_at)?;
        let outbox_expires_us = crate::time::parse_rfc3339_us(&c.outbox_expires_at)?;
        let evidence_observed_us = c
            .evidence
            .as_ref()
            .map(|e| crate::time::parse_rfc3339_us(&e.observed_at))
            .transpose()?;
        let trusted_now_us = self.clock.now_us()?;
        let trusted_now = crate::time::format_rfc3339_us(trusted_now_us);
        match (c.outcome, c.evidence.as_ref()) {
            (ObservedOutcome::VerifiedSuccess, Some(e)) if e.outcome == c.outcome => {}
            (ObservedOutcome::VerifiedFailure, Some(e))
                if e.outcome == c.outcome
                    && c.failure_code.as_deref().is_some_and(|s| !s.is_empty()) => {}
            (ObservedOutcome::Unknown, None) => {}
            (ObservedOutcome::Unknown, Some(e)) if e.outcome == c.outcome => {}
            (ObservedOutcome::VerifiedSuccess | ObservedOutcome::VerifiedFailure, _) => {
                return Err(StoreError::Invalid(
                    "verified outcome requires matching evidence".into(),
                ));
            }
            _ => {
                return Err(StoreError::Invalid(
                    "evidence outcome does not match attempt outcome".into(),
                ));
            }
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row: Option<(String, String, i64)> = tx.query_row(
            "SELECT o.account_id,o.current_status,a.worker_lease_generation FROM local_operation_journal o JOIN local_operation_attempt a ON a.operation_id=o.operation_id WHERE o.operation_id=?1 AND a.attempt_id=?2 AND a.attempt_status IN ('started','reconciliation_required','outcome_unknown') AND o.side_effect_may_have_occurred=1",
            params![c.operation_id, c.attempt_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).optional()?;
        let (account_id, operation_status, _) = row.ok_or_else(|| {
            StoreError::Invalid(
                "attempt is absent, terminal, or not marked as possibly effected".into(),
            )
        })?;
        if !matches!(
            operation_status.as_str(),
            "running" | "verifying" | "reconciliation_required" | "unknown_outcome"
        ) {
            return Err(StoreError::Invalid(
                "operation cannot accept an outcome from its current state".into(),
            ));
        }
        if let Some(e) = &c.evidence {
            if !(0..=3).contains(&e.sensitivity)
                || e.evidence_id.is_empty()
                || e.evidence_kind.is_empty()
                || e.reference.is_empty()
            {
                return Err(StoreError::Invalid(
                    "evidence metadata is incomplete or invalid".into(),
                ));
            }
            tx.execute(
                "INSERT INTO local_operation_evidence(evidence_id,operation_id,attempt_id,evidence_kind,observed_outcome,evidence_sha256,evidence_reference,sensitivity,observed_at,observed_at_us) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![e.evidence_id,c.operation_id,c.attempt_id,e.evidence_kind,outcome_key(e.outcome),e.sha256.as_slice(),e.reference,e.sensitivity,e.observed_at,evidence_observed_us],
            )?;
        }
        let (attempt_status, operation_status) = match c.outcome {
            ObservedOutcome::VerifiedSuccess => ("completed", "succeeded_verified"),
            ObservedOutcome::VerifiedFailure => ("failed", "failed"),
            ObservedOutcome::Unknown => ("outcome_unknown", "unknown_outcome"),
        };
        tx.execute(
            "UPDATE local_operation_attempt SET attempt_status=?1,completed_at=?2,completed_at_us=?3,failure_code=?4 WHERE attempt_id=?5 AND operation_id=?6",
            params![attempt_status,c.occurred_at,occurred_us,c.failure_code,c.attempt_id,c.operation_id],
        )?;
        tx.execute(
            "UPDATE local_operation_journal SET current_status=?1,updated_at=?2,updated_at_us=?3 WHERE operation_id=?4 AND side_effect_may_have_occurred=1",
            params![operation_status,trusted_now,trusted_now_us,c.operation_id],
        )?;
        let sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(aggregate_sequence),0)+1 FROM local_outbox WHERE account_id=?1 AND aggregate_type='operation' AND aggregate_id=?2",
            params![account_id,c.operation_id],
            |r| r.get(0),
        )?;
        let payload = serde_json::json!({
            "eventId": c.event_id,
            "operationId": c.operation_id,
            "attemptId": c.attempt_id,
            "outcome": outcome_key(c.outcome),
            "evidenceId": c.evidence.as_ref().map(|e| e.evidence_id.as_str()),
        });
        let payload_json =
            serde_json::to_string(&payload).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let payload_digest = crate::signatures::sha256(payload_json.as_bytes());
        let sensitivity = c.evidence.as_ref().map_or(1, |e| e.sensitivity);
        tx.execute(
            "INSERT INTO local_outbox(event_id,account_id,producer_id,source_event_id,event_type,schema_version,aggregate_type,aggregate_id,aggregate_sequence,correlation_id,occurred_at,payload_json,payload_reference,payload_digest_sha256,sensitivity,expires_at,delivery_state,occurred_at_us,expires_at_us) VALUES (?1,?2,'device-agent',?1,'lia.device.operation.outcome.v1','1','operation',?3,?4,?5,?6,?7,NULL,?8,?9,?10,'pending',?11,?12)",
            params![c.event_id,account_id,c.operation_id,sequence,c.correlation_id,c.occurred_at,payload_json,payload_digest.as_slice(),sensitivity,c.outbox_expires_at,occurred_us,outbox_expires_us],
        )?;
        Self::advance_integrity_anchor(&tx, self.anchor_path.as_deref(), trusted_now_us)?;
        tx.commit()?;
        Ok(())
    }
    pub fn operation_outcome(
        &self,
        operation_id: &str,
    ) -> Result<OperationOutcomeState, StoreError> {
        let row: Option<(String, i64, i64)> = self.conn.query_row(
            "SELECT current_status,side_effect_may_have_occurred,(SELECT count(*) FROM local_operation_attempt a WHERE a.operation_id=o.operation_id) FROM local_operation_journal o WHERE operation_id=?1",
            [operation_id],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional()?;
        let (status, may_have_effected, attempts) =
            row.ok_or_else(|| StoreError::Invalid("operation not found".into()))?;
        match status.as_str() {
            "succeeded_verified" => Ok(OperationOutcomeState::VerifiedSuccess),
            "failed" => Ok(OperationOutcomeState::VerifiedFailure),
            "unknown_outcome" | "reconciliation_required" | "running" | "verifying"
                if may_have_effected == 1 =>
            {
                Ok(OperationOutcomeState::Unknown)
            }
            _ if attempts == 0 => Ok(OperationOutcomeState::NotAttempted),
            _ => Ok(OperationOutcomeState::Unknown),
        }
    }
    pub fn insert_operation(&mut self, c: &OperationClaims) -> Result<(), StoreError> {
        let received_us = crate::time::parse_rfc3339_us(&c.received_at)?;
        let expires_us = crate::time::parse_rfc3339_us(&c.expires_at)?;
        if expires_us <= received_us {
            return Err(StoreError::Invalid(
                "operation expiry must follow receipt".into(),
            ));
        }
        self.conn.execute("INSERT INTO local_operation_journal(operation_id,account_id,target_device_id,target_credential_generation,capability_id,capability_schema_version,resource_type,resource_id,resource_generation,arguments_sha256,grant_reference,policy_version,risk_tier,current_status,received_at,expires_at,updated_at,revocation_epoch,purpose,originating_task_id,arguments_schema_uri,expected_resource_revision,resource_observed_at,resource_owning_device_id,received_at_us,expires_at_us,updated_at_us) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'accepted',?14,?15,?14,?16,?17,?18,?19,?20,?21,?22,?23,?24,?23)", params![c.operation_id,c.account_id,c.target_device_id,c.target_credential_generation,c.capability_id,c.capability_schema_version,c.resource_type,c.resource_id,c.resource_generation,c.arguments_sha256.as_slice(),c.grant_reference,c.policy_version,c.risk_tier,c.received_at,c.expires_at,c.revocation_epoch,c.purpose,c.originating_task_id,c.arguments_schema_uri,c.expected_resource_revision,c.resource_observed_at,c.resource_owning_device_id,received_us,expires_us])?;
        Ok(())
    }
    pub fn open_session(
        &mut self,
        session_id: &str,
        opened_at: &str,
        retain_until: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute("INSERT INTO local_session_sequence(session_id,last_session_sequence,updated_at,retain_until,has_accepted_sequence,updated_at_us,retain_until_us) VALUES (?1,0,?2,?3,0,?4,?5)",params![session_id,opened_at,retain_until,crate::time::parse_rfc3339_us(opened_at)?,crate::time::parse_rfc3339_us(retain_until)?])?;
        Ok(())
    }
    /// Call only after the caller has verified the signed CommandEnvelope, trust state,
    /// and equality between every signed operation claim and this stored operation.
    /// This crate currently provides the signature primitive, not that protobuf verifier.
    pub fn consume_command_before_effect(
        &mut self,
        op: &OperationClaims,
        command: &CommandClaims,
        attempt_id: &str,
        attempt_number: i64,
        worker_generation: i64,
        _at: &str,
    ) -> Result<EffectPermit, StoreError> {
        if command.session_sequence < 0 {
            return Err(StoreError::Invalid("negative command sequence".into()));
        }
        let nonce_expiry_us = crate::time::parse_rfc3339_us(&command.nonce_expires_at)?;
        let command_expiry_us = crate::time::parse_rfc3339_us(&command.expires_at)?;
        let now_us = self.clock.now_us()?;
        let trusted_at = crate::time::format_rfc3339_us(now_us);
        let retention_covers_expiry = i64::from(nonce_expiry_us >= command_expiry_us);
        if retention_covers_expiry != 1 {
            return Err(StoreError::Invalid(
                "nonce retention must cover signed command expiry".into(),
            ));
        }
        let in_window = i64::from(
            now_us >= crate::time::parse_rfc3339_us(&command.not_before)?
                && now_us < command_expiry_us,
        );
        if in_window != 1 {
            return Err(StoreError::Invalid(
                "command is outside its signed time window".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let n=tx.execute("INSERT INTO local_consumed_nonce(account_id,message_type,nonce,reference_id,consumed_at,expires_at,consumed_at_us,expires_at_us) VALUES (?1,'command',?2,?3,?4,?5,?6,?7)",params![op.account_id,command.nonce,command.command_id,trusted_at,command.nonce_expires_at,now_us,nonce_expiry_us])?;
        if n != 1 {
            return Err(StoreError::Invalid("nonce was not recorded".into()));
        }
        let seq=tx.execute("UPDATE local_session_sequence SET last_session_sequence=?1,has_accepted_sequence=1,updated_at=?2,updated_at_us=?3 WHERE session_id=?4 AND ((has_accepted_sequence=0 AND ?1=0) OR (has_accepted_sequence=1 AND ?1>last_session_sequence))",params![command.session_sequence,trusted_at,now_us,command.session_id])?;
        if seq != 1 {
            return Err(StoreError::Invalid(
                "unknown session or non-increasing command sequence".into(),
            ));
        }
        let changed=tx.execute("UPDATE local_operation_journal SET current_status='running',side_effect_may_have_occurred=1,updated_at=?1,updated_at_us=?5 WHERE operation_id=?2 AND account_id=?3 AND target_device_id=?4 AND current_status='accepted' AND expires_at_us>?5",params![trusted_at,op.operation_id,op.account_id,op.target_device_id,now_us])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "operation is absent, expired, or not accepted".into(),
            ));
        }
        tx.execute("INSERT INTO local_operation_attempt(attempt_id,command_id,operation_id,attempt_number,worker_lease_generation,attempt_status,started_at,started_at_us) VALUES (?1,?2,?3,?4,?5,'started',?6,?7)",params![attempt_id,command.command_id,op.operation_id,attempt_number,worker_generation,trusted_at,now_us])?;
        Self::advance_integrity_anchor(&tx, self.anchor_path.as_deref(), now_us)?;
        tx.commit()?;
        Ok(EffectPermit {
            operation_id: op.operation_id.clone(),
            attempt_id: attempt_id.into(),
            mode: "command",
        })
    }
    pub fn consume_lease_before_effect(
        &mut self,
        c: LeaseConsumption<'_>,
    ) -> Result<EffectPermit, StoreError> {
        let now_us = self.clock.now_us()?;
        let trusted_at = crate::time::format_rfc3339_us(now_us);
        let nonce_expiry_us = crate::time::parse_rfc3339_us(c.nonce_expires_at)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String)> = tx.query_row("SELECT operation_id,nonce FROM local_offline_lease WHERE lease_id=?1 AND account_id=?2 AND lease_state='stored' AND not_before_us<=?3 AND expires_at_us>?3",params![c.lease_id,c.account_id,now_us],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let (op, stored_nonce) =
            row.ok_or_else(|| StoreError::Invalid("lease not available or expired".into()))?;
        if stored_nonce != c.nonce {
            return Err(StoreError::Invalid("lease nonce mismatch".into()));
        }
        tx.execute("INSERT INTO local_consumed_nonce(account_id,message_type,nonce,reference_id,consumed_at,expires_at,consumed_at_us,expires_at_us) VALUES (?1,'offline_lease',?2,?3,?4,?5,?6,?7)",params![c.account_id,c.nonce,c.lease_id,trusted_at,c.nonce_expires_at,now_us,nonce_expiry_us])?;
        let changed=tx.execute("UPDATE local_operation_journal SET current_status='running',side_effect_may_have_occurred=1,updated_at=?1,updated_at_us=?4 WHERE operation_id=?2 AND account_id=?3 AND current_status='accepted' AND expires_at_us>?4",params![trusted_at,op,c.account_id,now_us])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "leased operation is absent, expired, or not accepted".into(),
            ));
        }
        tx.execute("INSERT INTO local_operation_attempt(attempt_id,command_id,operation_id,attempt_number,worker_lease_generation,attempt_status,started_at,started_at_us) VALUES (?1,?2,?3,?4,?5,'started',?6,?7)",params![c.attempt_id,c.lease_id,op,c.attempt_number,c.worker_generation,trusted_at,now_us])?;
        let changed=tx.execute("UPDATE local_offline_lease SET lease_state='consumed',consumed_at=?1,consumed_at_us=?4 WHERE lease_id=?2 AND account_id=?3 AND lease_state='stored'",params![trusted_at,c.lease_id,c.account_id,now_us])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "lease consume transition failed".into(),
            ));
        }
        Self::advance_integrity_anchor(&tx, self.anchor_path.as_deref(), now_us)?;
        tx.commit()?;
        Ok(EffectPermit {
            operation_id: op,
            attempt_id: c.attempt_id.into(),
            mode: "offline_lease",
        })
    }
    /// Call only after the caller has verified the signed lease, trust state, and time.
    /// SQL additionally binds the persisted lease claims to the exact operation row.
    pub fn insert_lease(
        &mut self,
        op: &OperationClaims,
        l: &LeaseClaims,
        received_at: &str,
    ) -> Result<(), StoreError> {
        if l.audience != op.target_device_id || l.signing_key_generation <= 0 {
            return Err(StoreError::Invalid(
                "lease audience must be the target device and key generation must be positive"
                    .into(),
            ));
        }
        let issued = crate::time::parse_rfc3339_us(&l.issued_at)?;
        let not_before = crate::time::parse_rfc3339_us(&l.not_before)?;
        let expires = crate::time::parse_rfc3339_us(&l.expires_at)?;
        let received = crate::time::parse_rfc3339_us(received_at)?;
        self.conn.execute("INSERT INTO local_offline_lease(lease_id,issuer_id,audience,operation_id,account_id,target_device_id,target_credential_generation,capability_id,capability_schema_version,resource_type,resource_id,resource_generation,resource_observed_revision,resource_observed_at,resource_owning_device_id,arguments_schema_uri,arguments_sha256,purpose,originating_task_id,grant_reference,policy_version,revocation_epoch,risk_tier,nonce,signing_key_generation,issued_at,not_before,expires_at,received_at,lease_state,issued_at_us,not_before_us,expires_at_us,received_at_us) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,'stored',?30,?31,?32,?33)",params![l.lease_id,l.issuer_id,l.audience,op.operation_id,op.account_id,op.target_device_id,op.target_credential_generation,op.capability_id,op.capability_schema_version,op.resource_type,op.resource_id,op.resource_generation,op.expected_resource_revision,op.resource_observed_at,op.resource_owning_device_id,op.arguments_schema_uri,op.arguments_sha256.as_slice(),op.purpose,op.originating_task_id,op.grant_reference,op.policy_version,op.revocation_epoch,op.risk_tier,l.nonce,l.signing_key_generation,l.issued_at,l.not_before,l.expires_at,received_at,issued,not_before,expires,received])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn op(id: &str) -> OperationClaims {
        OperationClaims {
            operation_id: id.into(),
            account_id: "acct".into(),
            target_device_id: "dev".into(),
            target_credential_generation: 1,
            capability_id: "cap".into(),
            capability_schema_version: 1,
            resource_type: "window".into(),
            resource_id: "w1".into(),
            resource_generation: "gen1".into(),
            expected_resource_revision: Some("rev1".into()),
            resource_observed_at: Some("2026-11-15T00:00:00Z".into()),
            resource_owning_device_id: Some("dev".into()),
            arguments_schema_uri: "urn:args:v1".into(),
            arguments_sha256: [7; 32],
            grant_reference: "grant".into(),
            policy_version: 1,
            revocation_epoch: 0,
            risk_tier: 0,
            purpose: "local-control".into(),
            originating_task_id: None,
            received_at: "2026-11-15T00:00:01Z".into(),
            expires_at: "2026-11-15T00:05:00Z".into(),
        }
    }
    fn command(id: &str, seq: i64) -> CommandClaims {
        CommandClaims {
            command_id: id.into(),
            nonce: format!("nonce-{id}"),
            session_id: "session".into(),
            session_sequence: seq,
            not_before: "2026-11-15T00:00:00Z".into(),
            expires_at: "2026-11-15T00:05:00Z".into(),
            nonce_expires_at: "2026-11-15T00:10:00Z".into(),
        }
    }
    #[test]
    fn migrations_wal_foreign_keys_and_first_zero_once() {
        let mut s = AgentStore::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), 8);
        assert_eq!(s.pragma_i64("foreign_keys").unwrap(), 1);
        assert_eq!(
            s.pragma_i64("synchronous").unwrap(),
            2,
            "SQLite FULL durability is synchronous=2"
        );
        // SQLite reports memory journal mode as memory; the file-backed WAL assertion is separate in CI.
        s.open_session("session", "2026-11-15T00:00:00Z", "2026-11-15T00:10:10Z")
            .unwrap();
        s.insert_operation(&op("o1")).unwrap();
        let p = s
            .consume_command_before_effect(
                &op("o1"),
                &command("c0", 0),
                "a0",
                1,
                1,
                "2026-11-15T00:00:02Z",
            )
            .unwrap();
        assert_eq!(p.mode(), "command");
        // Once the operation is committed as possibly effected it cannot be re-entered.
        assert!(
            s.consume_command_before_effect(
                &op("o1"),
                &command("c1", 1),
                "a1",
                2,
                1,
                "2026-11-15T00:00:03Z"
            )
            .is_err()
        );
        assert!(s.conn.query_row("SELECT last_session_sequence FROM local_session_sequence WHERE session_id='session'",[],|r|r.get::<_,i64>(0)).unwrap()==0);
    }
    #[test]
    fn migration_008_backfills_legacy_microseconds_and_removes_julianday_checks() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (_, sql) in MIGRATIONS.iter().take(7) {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute("INSERT INTO local_session_sequence(session_id,last_session_sequence,updated_at,retain_until,has_accepted_sequence) VALUES ('legacy',0,'2026-11-15T00:00:01.123456Z','2026-11-15T00:10:01.123456Z',0)", []).unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tx.execute_batch("DROP TRIGGER IF EXISTS local_session_sequence_monotonic;")
            .unwrap();
        backfill_integer_times(&tx).unwrap();
        tx.execute_batch(MIGRATIONS[7].1).unwrap();
        tx.commit().unwrap();
        let (updated, expiry): (i64,i64) = conn.query_row("SELECT updated_at_us,retain_until_us FROM local_session_sequence WHERE session_id='legacy'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(
            updated,
            crate::time::parse_rfc3339_us("2026-11-15T00:00:01.123456Z").unwrap()
        );
        assert_eq!(
            expiry,
            crate::time::parse_rfc3339_us("2026-11-15T00:10:01.123456Z").unwrap()
        );
        let legacy_time_sql: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE sql LIKE '%julianday%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            legacy_time_sql, 0,
            "migrated schema must not retain floating-point time checks or triggers"
        );
    }
    #[test]
    fn file_database_uses_wal_and_applies_entire_migration_chain() {
        let path =
            std::env::temp_dir().join(format!("lia-device-agent-{}.sqlite", std::process::id()));
        clear_database_files(&path);
        let s = AgentStore::open(&path).unwrap();
        let mode: String = s
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        assert_eq!(s.schema_version().unwrap(), 8);
        drop(s);
        clear_database_files(&path);
    }
    #[test]
    fn concurrent_first_open_serializes_migrations() {
        let path = std::env::temp_dir().join(format!(
            "lia-agent-concurrent-{}.sqlite",
            std::process::id()
        ));
        clear_database_files(&path);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    AgentStore::open(path).and_then(|store| store.schema_version())
                })
            })
            .collect();
        barrier.wait();
        for handle in handles {
            assert_eq!(handle.join().unwrap().unwrap(), 8);
        }
        clear_database_files(&path);
    }
    #[test]
    fn backup_restore_preserves_replay_barrier_and_restart_reconciles_effects() {
        let base = std::env::temp_dir().join(format!("lia-agent-recovery-{}", std::process::id()));
        let source = base.with_extension("source.sqlite");
        let backup = base.with_extension("backup.sqlite");
        let restored = base.with_extension("restored.sqlite");
        for path in [&source, &backup, &restored] {
            clear_database_files(path);
        }
        {
            let mut store = AgentStore::open(&source).unwrap();
            store
                .open_session("session", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
                .unwrap();
            store.insert_operation(&op("recovery-op")).unwrap();
            store
                .consume_command_before_effect(
                    &op("recovery-op"),
                    &command("recovery-command", 0),
                    "recovery-attempt",
                    1,
                    1,
                    "2026-11-15T00:00:02Z",
                )
                .unwrap();
            store.backup_to(&backup).unwrap();
        }
        {
            let mut restore_target = AgentStore::open(&restored).unwrap();
            std::fs::copy(anchor_path(&source), anchor_path(&restored)).unwrap();
            restore_target.restore_from(&backup).unwrap();
            assert_eq!(restore_target.schema_version().unwrap(), 8);
            assert_eq!(
                restore_target
                    .reconcile_uncertain_operations("2026-11-15T00:01:00Z")
                    .unwrap(),
                1
            );
            let (status, marker): (String, i64) = restore_target.conn.query_row("SELECT current_status,side_effect_may_have_occurred FROM local_operation_journal WHERE operation_id='recovery-op'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
            assert_eq!((status.as_str(), marker), ("reconciliation_required", 1));
            assert!(
                restore_target
                    .consume_command_before_effect(
                        &op("recovery-op"),
                        &command("new-command", 1),
                        "new-attempt",
                        2,
                        1,
                        "2026-11-15T00:01:01Z"
                    )
                    .is_err()
            );
            let attempt_status: String = restore_target.conn.query_row("SELECT attempt_status FROM local_operation_attempt WHERE attempt_id='recovery-attempt'", [], |r| r.get(0)).unwrap();
            assert_eq!(attempt_status, "reconciliation_required");
        }
        for path in [&source, &backup, &restored] {
            clear_database_files(path);
        }
    }

    #[test]
    fn stale_database_restore_is_rejected_by_external_anchor() {
        let path =
            std::env::temp_dir().join(format!("lia-stale-restore-{}.sqlite", std::process::id()));
        let backup = path.with_extension("stale-backup.sqlite");
        clear_database_files(&path);
        let _ = std::fs::remove_file(&backup);
        let mut store = AgentStore::open(&path).unwrap();
        store
            .open_session("session", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        store.insert_operation(&op("restore-one")).unwrap();
        store.insert_operation(&op("restore-two")).unwrap();
        store
            .consume_command_before_effect(
                &op("restore-one"),
                &command("restore-c1", 0),
                "restore-a1",
                1,
                1,
                "2026-11-15T00:00:02Z",
            )
            .unwrap();
        store.backup_to(&backup).unwrap();
        store
            .consume_command_before_effect(
                &op("restore-two"),
                &command("restore-c2", 1),
                "restore-a2",
                2,
                1,
                "2026-11-15T00:00:03Z",
            )
            .unwrap();
        assert!(
            store.restore_from(&backup).is_err(),
            "restoring a lower protected-write sequence must fail closed"
        );
        assert!(
            store
                .consume_command_before_effect(
                    &op("restore-two"),
                    &command("restore-replay", 1),
                    "restore-a3",
                    3,
                    1,
                    "2026-11-15T00:00:03Z"
                )
                .is_err()
        );
        drop(store);
        assert!(
            AgentStore::open(&path).is_err(),
            "startup must reject the stale DB/anchor pair"
        );
        clear_database_files(&path);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn external_clock_high_water_rejects_rollback() {
        let path =
            std::env::temp_dir().join(format!("lia-clock-rollback-{}.sqlite", std::process::id()));
        clear_database_files(&path);
        let now = crate::time::parse_rfc3339_us("2026-11-15T00:03:00Z").unwrap();
        let clock: Arc<dyn TrustedClock> = Arc::new(FixedClock(now));
        let mut store = AgentStore::open_with_clock(&path, clock).unwrap();
        store
            .open_session("session", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        store.insert_operation(&op("clock-op")).unwrap();
        store
            .consume_command_before_effect(
                &op("clock-op"),
                &command("clock-cmd", 0),
                "clock-attempt",
                1,
                1,
                "2026-11-15T00:00:02Z",
            )
            .unwrap();
        drop(store);
        let rolled_back: Arc<dyn TrustedClock> = Arc::new(FixedClock(now - 1));
        assert!(AgentStore::open_with_clock(&path, rolled_back).is_err());
        clear_database_files(&path);
    }
    #[test]
    fn increasing_command_consumes_nonce_and_effect_marker_atomically() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.open_session("session", "2026-11-15T00:00:00Z", "2026-11-15T00:10:10Z")
            .unwrap();
        s.insert_operation(&op("o1")).unwrap();
        s.insert_operation(&op("o2")).unwrap();
        s.consume_command_before_effect(
            &op("o1"),
            &command("c0", 0),
            "a0",
            1,
            1,
            "2026-11-15T00:00:02Z",
        )
        .unwrap();
        let p = s
            .consume_command_before_effect(
                &op("o2"),
                &command("c1", 1),
                "a1",
                1,
                1,
                "2026-11-15T00:00:03Z",
            )
            .unwrap();
        assert_eq!(p.operation_id(), "o2");
        assert!(
            s.consume_command_before_effect(
                &op("o2"),
                &command("c2", 2),
                "a2",
                2,
                1,
                "2026-11-15T00:00:04Z"
            )
            .is_err()
        );
        let marker:i64=s.conn.query_row("SELECT side_effect_may_have_occurred FROM local_operation_journal WHERE operation_id='o2'",[],|r|r.get(0)).unwrap();
        assert_eq!(marker, 1);
        let count: i64 = s
            .conn
            .query_row(
                "SELECT count(*) FROM local_consumed_nonce WHERE account_id='acct'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }
    #[test]
    fn failed_attempt_insert_rolls_back_nonce_watermark_and_effect_marker() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.open_session("session", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        s.insert_operation(&op("existing")).unwrap();
        s.insert_operation(&op("target")).unwrap();
        s.conn.execute("INSERT INTO local_operation_attempt(attempt_id,command_id,operation_id,attempt_number,worker_lease_generation,attempt_status) VALUES ('prior','collide','existing',1,1,'created')", []).unwrap();
        let mut colliding = command("collide", 0);
        colliding.nonce = "new-nonce".into();
        assert!(
            s.consume_command_before_effect(
                &op("target"),
                &colliding,
                "attempt",
                1,
                1,
                "2026-11-15T00:00:02Z"
            )
            .is_err()
        );
        let (status, marker): (String, i64) = s.conn.query_row("SELECT current_status,side_effect_may_have_occurred FROM local_operation_journal WHERE operation_id='target'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!((status.as_str(), marker), ("accepted", 0));
        let (sequence, accepted): (i64, i64) = s.conn.query_row("SELECT last_session_sequence,has_accepted_sequence FROM local_session_sequence WHERE session_id='session'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!((sequence, accepted), (0, 0));
        let nonce_count: i64 = s
            .conn
            .query_row("SELECT count(*) FROM local_consumed_nonce", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(nonce_count, 0);
    }
    #[test]
    fn command_nonce_replay_is_rejected_even_in_a_new_session() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.open_session("session-a", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        s.open_session("session-b", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        s.insert_operation(&op("first")).unwrap();
        s.insert_operation(&op("second")).unwrap();
        let mut first = command("command-a", 0);
        first.session_id = "session-a".into();
        first.nonce = "same-nonce".into();
        s.consume_command_before_effect(
            &op("first"),
            &first,
            "attempt-a",
            1,
            1,
            "2026-11-15T00:00:02Z",
        )
        .unwrap();
        let mut replay = command("command-b", 0);
        replay.session_id = "session-b".into();
        replay.nonce = "same-nonce".into();
        assert!(
            s.consume_command_before_effect(
                &op("second"),
                &replay,
                "attempt-b",
                1,
                1,
                "2026-11-15T00:00:03Z"
            )
            .is_err()
        );
        let (sequence, accepted): (i64, i64) = s.conn.query_row("SELECT last_session_sequence,has_accepted_sequence FROM local_session_sequence WHERE session_id='session-b'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!((sequence, accepted), (0, 0));
    }
    #[test]
    fn lease_claim_binding_and_consume_before_effect() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.insert_operation(&op("leased")).unwrap();
        let lease = LeaseClaims {
            lease_id: "lease".into(),
            issuer_id: "coordinator".into(),
            audience: "dev".into(),
            nonce: "lease-nonce".into(),
            signing_key_generation: 1,
            issued_at: "2026-11-15T00:00:00Z".into(),
            not_before: "2026-11-15T00:00:00Z".into(),
            expires_at: "2026-11-15T00:04:00Z".into(),
        };
        let mut wrong_audience = lease.clone();
        wrong_audience.audience = "another-device".into();
        assert!(
            s.insert_lease(&op("leased"), &wrong_audience, "2026-11-15T00:00:01Z")
                .is_err()
        );
        let mut wrong = op("leased");
        wrong.arguments_sha256 = [8; 32];
        assert!(
            s.insert_lease(&wrong, &lease, "2026-11-15T00:00:01Z")
                .is_err()
        );
        let mut wrong = op("leased");
        wrong.resource_id = "other-resource".into();
        assert!(
            s.insert_lease(&wrong, &lease, "2026-11-15T00:00:01Z")
                .is_err()
        );
        let mut wrong = op("leased");
        wrong.target_device_id = "other-device".into();
        assert!(
            s.insert_lease(&wrong, &lease, "2026-11-15T00:00:01Z")
                .is_err()
        );
        s.insert_lease(&op("leased"), &lease, "2026-11-15T00:00:01Z")
            .unwrap();
        let consume = |attempt_id| LeaseConsumption {
            account_id: "acct",
            lease_id: "lease",
            nonce: "lease-nonce",
            attempt_id,
            attempt_number: 1,
            worker_generation: 1,
            at: "2026-11-15T00:00:02Z",
            nonce_expires_at: "2026-11-15T00:10:00Z",
        };
        let permit = s.consume_lease_before_effect(consume("attempt")).unwrap();
        assert_eq!(permit.mode(), "offline_lease");
        assert!(s.consume_lease_before_effect(consume("attempt2")).is_err());
        let state: String = s
            .conn
            .query_row(
                "SELECT lease_state FROM local_offline_lease WHERE lease_id='lease'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(state, "consumed");
    }
    #[test]
    fn finish_attempt_records_verified_and_unknown_outcomes_atomically() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.open_session("session", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        for id in ["success", "failure", "unknown", "not-attempted"] {
            s.insert_operation(&op(id)).unwrap();
        }
        for (index, id) in ["success", "failure", "unknown"].iter().enumerate() {
            let command_id = format!("cmd-{id}");
            s.consume_command_before_effect(
                &op(id),
                &command(&command_id, index as i64),
                &format!("attempt-{id}"),
                1,
                1,
                "2026-11-15T00:00:02Z",
            )
            .unwrap();
        }
        let evidence = |id: &str, outcome| EvidenceRecord {
            evidence_id: format!("evidence-{id}"),
            evidence_kind: "adapter-readback".into(),
            outcome,
            sha256: [9; 32],
            reference: format!("local-evidence://{id}"),
            sensitivity: 0,
            observed_at: "2026-11-15T00:00:03Z".into(),
        };
        let complete = |id: &str, outcome, evidence, failure_code| AttemptCompletion {
            operation_id: id.into(),
            attempt_id: format!("attempt-{id}"),
            event_id: format!("event-{id}"),
            correlation_id: format!("correlation-{id}"),
            occurred_at: "2026-11-15T00:00:04Z".into(),
            outbox_expires_at: "2026-11-16T00:00:00Z".into(),
            outcome,
            failure_code,
            evidence,
        };
        assert!(
            s.finish_attempt(&complete(
                "success",
                ObservedOutcome::VerifiedSuccess,
                None,
                None
            ))
            .is_err()
        );
        s.finish_attempt(&complete(
            "success",
            ObservedOutcome::VerifiedSuccess,
            Some(evidence("success", ObservedOutcome::VerifiedSuccess)),
            None,
        ))
        .unwrap();
        s.finish_attempt(&complete(
            "failure",
            ObservedOutcome::VerifiedFailure,
            Some(evidence("failure", ObservedOutcome::VerifiedFailure)),
            Some("readback-mismatch".into()),
        ))
        .unwrap();
        s.finish_attempt(&complete("unknown", ObservedOutcome::Unknown, None, None))
            .unwrap();
        assert_eq!(
            s.operation_outcome("not-attempted").unwrap(),
            OperationOutcomeState::NotAttempted
        );
        assert_eq!(
            s.operation_outcome("success").unwrap(),
            OperationOutcomeState::VerifiedSuccess
        );
        assert_eq!(
            s.operation_outcome("failure").unwrap(),
            OperationOutcomeState::VerifiedFailure
        );
        assert_eq!(
            s.operation_outcome("unknown").unwrap(),
            OperationOutcomeState::Unknown
        );
        let (outbox_count, evidence_count): (i64,i64) = s.conn.query_row("SELECT (SELECT count(*) FROM local_outbox),(SELECT count(*) FROM local_operation_evidence)", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!((outbox_count, evidence_count), (3, 2));
        let (payload,digest): (String,Vec<u8>) = s.conn.query_row("SELECT payload_json,payload_digest_sha256 FROM local_outbox WHERE event_id='event-success'", [], |r| Ok((r.get(0)?,r.get(1)?))).unwrap();
        assert_eq!(
            digest.as_slice(),
            crate::signatures::sha256(payload.as_bytes())
        );
        assert!(s.conn.execute("UPDATE local_operation_evidence SET evidence_reference='tampered' WHERE evidence_id='evidence-success'", []).is_err());
        assert!(s.conn.execute("UPDATE local_operation_journal SET current_status='succeeded_verified' WHERE operation_id='not-attempted'", []).is_err());
    }
    #[test]
    fn sequence_zero_update_then_monotonic_and_retention_immutable() {
        let mut s = AgentStore::open_in_memory().unwrap();
        s.open_session("s", "2026-11-15T00:00:00Z", "2099-11-15T00:10:10Z")
            .unwrap();
        s.conn.execute("UPDATE local_session_sequence SET has_accepted_sequence=1,updated_at='2026-11-15T00:00:01Z' WHERE session_id='s'",[]).unwrap();
        assert!(s.conn.execute("UPDATE local_session_sequence SET last_session_sequence=0 WHERE session_id='s'",[]).is_err());
        s.conn.execute("UPDATE local_session_sequence SET last_session_sequence=1,updated_at='2026-11-15T00:00:02Z' WHERE session_id='s'",[]).unwrap();
        assert!(s.conn.execute("UPDATE local_session_sequence SET last_session_sequence=0 WHERE session_id='s'",[]).is_err());
        assert!(s.conn.execute("UPDATE local_session_sequence SET retain_until='2026-11-15T00:20:00Z' WHERE session_id='s'",[]).is_err());
        assert!(
            s.conn
                .execute(
                    "DELETE FROM local_session_sequence WHERE session_id='s'",
                    []
                )
                .is_err()
        );
    }

    #[test]
    fn process_kill_recovery_windows() {
        if let Ok(path) = std::env::var("LIA_KILL_TEST_COMMIT_PATH") {
            let mut store = AgentStore::open(&path).unwrap();
            store
                .open_session("session", "2026-11-15T00:00:00Z", "2026-11-15T00:10:10Z")
                .unwrap();
            store.insert_operation(&op("killed-operation")).unwrap();
            store
                .consume_command_before_effect(
                    &op("killed-operation"),
                    &command("killed-command", 0),
                    "killed-attempt",
                    1,
                    1,
                    "2026-11-15T00:00:02Z",
                )
                .unwrap();
            // Simulate process death after the durable consume commit and before adapter effect.
            std::process::exit(0);
        }
        if let Ok(path) = std::env::var("LIA_KILL_TEST_TX_PATH") {
            let conn = Connection::open(path).unwrap();
            conn.execute_batch("CREATE TABLE kill_test(value INTEGER); INSERT INTO kill_test VALUES(0); BEGIN IMMEDIATE; UPDATE kill_test SET value=1;").unwrap();
            // exit() skips Rust destructors, leaving SQLite to recover the uncommitted WAL txn.
            std::process::exit(0);
        }

        use std::process::Command;
        let bin = std::env::current_exe().unwrap();
        let commit_path =
            std::env::temp_dir().join(format!("lia-kill-commit-{}.sqlite", std::process::id()));
        clear_database_files(&commit_path);
        let status = Command::new(&bin)
            .arg("store::tests::process_kill_recovery_windows")
            .arg("--exact")
            .env("LIA_KILL_TEST_COMMIT_PATH", &commit_path)
            .status()
            .unwrap();
        assert!(status.success());
        let mut store = AgentStore::open(&commit_path).unwrap();
        let state: String = store.conn.query_row("SELECT current_status FROM local_operation_journal WHERE operation_id='killed-operation'", [], |r| r.get(0)).unwrap();
        assert_eq!(state, "running");
        assert_eq!(
            store
                .reconcile_uncertain_operations("2026-11-15T00:01:00Z")
                .unwrap(),
            1
        );
        let _ = std::fs::remove_file(&commit_path);
        let _ = std::fs::remove_file(commit_path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(commit_path.with_extension("sqlite-shm"));

        let txn_path =
            std::env::temp_dir().join(format!("lia-kill-txn-{}.sqlite", std::process::id()));
        clear_database_files(&txn_path);
        drop(AgentStore::open(&txn_path).unwrap());
        let status = Command::new(bin)
            .arg("store::tests::process_kill_recovery_windows")
            .arg("--exact")
            .env("LIA_KILL_TEST_TX_PATH", &txn_path)
            .status()
            .unwrap();
        assert!(status.success());
        let store = AgentStore::open(&txn_path).unwrap();
        let value: i64 = store
            .conn
            .query_row("SELECT value FROM kill_test", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            value, 0,
            "uncommitted write must roll back after process termination"
        );
        clear_database_files(&txn_path);
    }

    #[test]
    fn wal_readers_contending_writers_busy_and_checkpoint_progress() {
        let path =
            std::env::temp_dir().join(format!("lia-wal-contention-{}.sqlite", std::process::id()));
        clear_database_files(&path);
        let store = AgentStore::open(&path).unwrap();
        let reader = Connection::open(&path).unwrap();
        reader
            .execute_batch("BEGIN; SELECT last_trusted_time_us FROM local_trusted_clock_state;")
            .unwrap();
        let second_reader = Connection::open(&path).unwrap();
        second_reader
            .execute_batch("BEGIN; SELECT anchor_sequence FROM local_integrity_state;")
            .unwrap();
        let writer = Connection::open(&path).unwrap();
        writer.busy_timeout(Duration::from_millis(20)).unwrap();
        writer.execute_batch("BEGIN IMMEDIATE;").unwrap();
        let contender = Connection::open(&path).unwrap();
        contender.busy_timeout(Duration::from_millis(10)).unwrap();
        assert!(
            contender.execute_batch("BEGIN IMMEDIATE;").is_err(),
            "contending writer should report SQLITE_BUSY within its timeout"
        );
        writer.execute_batch("UPDATE local_trusted_clock_state SET last_trusted_time_us=last_trusted_time_us+1; COMMIT;").unwrap();
        let wal_path = path.with_file_name(format!(
            "{}-wal",
            path.file_name().unwrap().to_string_lossy()
        ));
        assert!(
            std::fs::metadata(&wal_path).unwrap().len() > 0,
            "long-lived reader should pin WAL frames"
        );
        let (busy, log_frames, checkpointed): (i64, i64, i64) = store
            .conn
            .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert!(busy == 0 || checkpointed <= log_frames);
        reader.execute_batch("ROLLBACK;").unwrap();
        second_reader.execute_batch("ROLLBACK;").unwrap();
        let (_, log_after, checkpointed_after): (i64, i64, i64) = store
            .conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(log_after, 0);
        assert_eq!(checkpointed_after, 0);
        drop(reader);
        drop(second_reader);
        drop(writer);
        drop(contender);
        drop(store);
        clear_database_files(&path);
    }
}
