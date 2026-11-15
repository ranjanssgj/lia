use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::{path::Path, time::Duration};

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
];

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
}
impl AgentStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Self::configure(conn)
    }
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::configure(Connection::open_in_memory()?)
    }
    fn configure(conn: Connection) -> Result<Self, StoreError> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch("CREATE TABLE IF NOT EXISTS lia_schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')));")?;
        for (version, sql) in MIGRATIONS {
            let exists: Option<i64> = conn
                .query_row(
                    "SELECT version FROM lia_schema_migrations WHERE version=?1",
                    [version],
                    |r| r.get(0),
                )
                .optional()?;
            if exists.is_none() {
                let tx = conn.unchecked_transaction()?;
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO lia_schema_migrations(version) VALUES (?1)",
                    [version],
                )?;
                tx.commit()?;
            }
        }
        Ok(Self { conn })
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
        Ok(())
    }
    /// On startup after an interrupted process, quarantine all committed in-flight effects.
    /// It preserves the possible-effect marker and never returns the operation to accepted.
    pub fn reconcile_uncertain_operations(&mut self, at: &str) -> Result<usize, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count = tx.execute(
            "UPDATE local_operation_journal SET current_status='reconciliation_required',updated_at=?1 WHERE current_status='running' AND side_effect_may_have_occurred=1",
            [at],
        )?;
        tx.execute(
            "UPDATE local_operation_attempt SET attempt_status='reconciliation_required' WHERE attempt_status='started' AND operation_id IN (SELECT operation_id FROM local_operation_journal WHERE current_status='reconciliation_required' AND side_effect_may_have_occurred=1)",
            [],
        )?;
        tx.commit()?;
        Ok(count)
    }
    pub fn insert_operation(&mut self, c: &OperationClaims) -> Result<(), StoreError> {
        self.conn.execute("INSERT INTO local_operation_journal(operation_id,account_id,target_device_id,target_credential_generation,capability_id,capability_schema_version,resource_type,resource_id,resource_generation,arguments_sha256,grant_reference,policy_version,risk_tier,current_status,received_at,expires_at,updated_at,revocation_epoch,purpose,originating_task_id,arguments_schema_uri,expected_resource_revision,resource_observed_at,resource_owning_device_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'accepted',?14,?15,?14,?16,?17,?18,?19,?20,?21,?22)", params![c.operation_id,c.account_id,c.target_device_id,c.target_credential_generation,c.capability_id,c.capability_schema_version,c.resource_type,c.resource_id,c.resource_generation,c.arguments_sha256.as_slice(),c.grant_reference,c.policy_version,c.risk_tier,c.received_at,c.expires_at,c.revocation_epoch,c.purpose,c.originating_task_id,c.arguments_schema_uri,c.expected_resource_revision,c.resource_observed_at,c.resource_owning_device_id])?;
        Ok(())
    }
    pub fn open_session(
        &mut self,
        session_id: &str,
        opened_at: &str,
        retain_until: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute("INSERT INTO local_session_sequence(session_id,last_session_sequence,updated_at,retain_until,has_accepted_sequence) VALUES (?1,0,?2,?3,0)",params![session_id,opened_at,retain_until])?;
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
        at: &str,
    ) -> Result<EffectPermit, StoreError> {
        if command.session_sequence < 0 {
            return Err(StoreError::Invalid("negative command sequence".into()));
        }
        let retention_covers_expiry: i64 = self.conn.query_row(
            "SELECT julianday(?1)>=julianday(?2)",
            params![command.nonce_expires_at, command.expires_at],
            |r| r.get(0),
        )?;
        if retention_covers_expiry != 1 {
            return Err(StoreError::Invalid(
                "nonce retention must cover signed command expiry".into(),
            ));
        }
        let in_window: i64 = self.conn.query_row(
            "SELECT (julianday(?1)>=julianday(?2) AND julianday(?1)<julianday(?3))",
            params![at, command.not_before, command.expires_at],
            |r| r.get(0),
        )?;
        if in_window != 1 {
            return Err(StoreError::Invalid(
                "command is outside its signed time window".into(),
            ));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let n=tx.execute("INSERT INTO local_consumed_nonce(account_id,message_type,nonce,reference_id,consumed_at,expires_at) VALUES (?1,'command',?2,?3,?4,?5)",params![op.account_id,command.nonce,command.command_id,at,command.nonce_expires_at])?;
        if n != 1 {
            return Err(StoreError::Invalid("nonce was not recorded".into()));
        }
        let seq=tx.execute("UPDATE local_session_sequence SET last_session_sequence=?1,has_accepted_sequence=1,updated_at=?2 WHERE session_id=?3 AND ((has_accepted_sequence=0 AND ?1=0) OR (has_accepted_sequence=1 AND ?1>last_session_sequence))",params![command.session_sequence,at,command.session_id])?;
        if seq != 1 {
            return Err(StoreError::Invalid(
                "unknown session or non-increasing command sequence".into(),
            ));
        }
        let changed=tx.execute("UPDATE local_operation_journal SET current_status='running',side_effect_may_have_occurred=1,updated_at=?1 WHERE operation_id=?2 AND account_id=?3 AND target_device_id=?4 AND current_status='accepted' AND julianday(expires_at)>julianday(?1)",params![at,op.operation_id,op.account_id,op.target_device_id])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "operation is absent, expired, or not accepted".into(),
            ));
        }
        tx.execute("INSERT INTO local_operation_attempt(attempt_id,command_id,operation_id,attempt_number,worker_lease_generation,attempt_status,started_at) VALUES (?1,?2,?3,?4,?5,'started',?6)",params![attempt_id,command.command_id,op.operation_id,attempt_number,worker_generation,at])?;
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
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String)> = tx.query_row("SELECT operation_id,nonce FROM local_offline_lease WHERE lease_id=?1 AND account_id=?2 AND lease_state='stored' AND julianday(not_before)<=julianday(?3) AND julianday(expires_at)>julianday(?3)",params![c.lease_id,c.account_id,c.at],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let (op, stored_nonce) =
            row.ok_or_else(|| StoreError::Invalid("lease not available or expired".into()))?;
        if stored_nonce != c.nonce {
            return Err(StoreError::Invalid("lease nonce mismatch".into()));
        }
        tx.execute("INSERT INTO local_consumed_nonce(account_id,message_type,nonce,reference_id,consumed_at,expires_at) VALUES (?1,'offline_lease',?2,?3,?4,?5)",params![c.account_id,c.nonce,c.lease_id,c.at,c.nonce_expires_at])?;
        let changed=tx.execute("UPDATE local_operation_journal SET current_status='running',side_effect_may_have_occurred=1,updated_at=?1 WHERE operation_id=?2 AND account_id=?3 AND current_status='accepted' AND julianday(expires_at)>julianday(?1)",params![c.at,op,c.account_id])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "leased operation is absent, expired, or not accepted".into(),
            ));
        }
        tx.execute("INSERT INTO local_operation_attempt(attempt_id,command_id,operation_id,attempt_number,worker_lease_generation,attempt_status,started_at) VALUES (?1,?2,?3,?4,?5,'started',?6)",params![c.attempt_id,c.lease_id,op,c.attempt_number,c.worker_generation,c.at])?;
        let changed=tx.execute("UPDATE local_offline_lease SET lease_state='consumed',consumed_at=?1 WHERE lease_id=?2 AND account_id=?3 AND lease_state='stored'",params![c.at,c.lease_id,c.account_id])?;
        if changed != 1 {
            return Err(StoreError::Invalid(
                "lease consume transition failed".into(),
            ));
        }
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
        self.conn.execute("INSERT INTO local_offline_lease(lease_id,issuer_id,audience,operation_id,account_id,target_device_id,target_credential_generation,capability_id,capability_schema_version,resource_type,resource_id,resource_generation,resource_observed_revision,resource_observed_at,resource_owning_device_id,arguments_schema_uri,arguments_sha256,purpose,originating_task_id,grant_reference,policy_version,revocation_epoch,risk_tier,nonce,signing_key_generation,issued_at,not_before,expires_at,received_at,lease_state) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,'stored')",params![l.lease_id,l.issuer_id,l.audience,op.operation_id,op.account_id,op.target_device_id,op.target_credential_generation,op.capability_id,op.capability_schema_version,op.resource_type,op.resource_id,op.resource_generation,op.expected_resource_revision,op.resource_observed_at,op.resource_owning_device_id,op.arguments_schema_uri,op.arguments_sha256.as_slice(),op.purpose,op.originating_task_id,op.grant_reference,op.policy_version,op.revocation_epoch,op.risk_tier,l.nonce,l.signing_key_generation,l.issued_at,l.not_before,l.expires_at,received_at])?;
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
        assert_eq!(s.schema_version().unwrap(), 5);
        assert_eq!(s.pragma_i64("foreign_keys").unwrap(), 1);
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
    fn file_database_uses_wal_and_applies_entire_migration_chain() {
        let path =
            std::env::temp_dir().join(format!("lia-device-agent-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let s = AgentStore::open(&path).unwrap();
        let mode: String = s
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        assert_eq!(s.schema_version().unwrap(), 5);
        drop(s);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
    }
    #[test]
    fn backup_restore_preserves_replay_barrier_and_restart_reconciles_effects() {
        let base = std::env::temp_dir().join(format!("lia-agent-recovery-{}", std::process::id()));
        let source = base.with_extension("source.sqlite");
        let backup = base.with_extension("backup.sqlite");
        let restored = base.with_extension("restored.sqlite");
        for path in [&source, &backup, &restored] {
            let _ = std::fs::remove_file(path);
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
            restore_target.restore_from(&backup).unwrap();
            assert_eq!(restore_target.schema_version().unwrap(), 5);
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
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
            let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
        }
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
}
