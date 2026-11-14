-- LIA P4 draft SQLite migration 003: single-use offline leases and replay ledgers.
-- Apply after 001/002. The ledger is device-local and persists across app restart/rollback.
-- Verify signature, trust/key generation, local policy, time confidence, resource generation,
-- and argument digest before execution.
--
-- Signed CommandEnvelope path (it carries session_sequence):
--   BEGIN IMMEDIATE;
--   1. Compare-and-swap local_session_sequence so incoming sequence > stored sequence.
--   2. Insert the command nonce into local_consumed_nonce.
--   3. Mark the operation running + side_effect_may_have_occurred=1.
--   COMMIT. Only after COMMIT may the adapter effect begin.
--
-- OfflineAuthorizationLease path (it has a lease nonce but no session_sequence field):
--   BEGIN IMMEDIATE;
--   1. Insert the offline-lease nonce/reference into local_consumed_nonce.
--   2. Mark the operation running + side_effect_may_have_occurred=1.
--   3. Transition the exact stored lease to consumed.
--   COMMIT. Only after COMMIT may the adapter effect begin.
--
-- A failed check/UNIQUE violation aborts and fails closed. An uncertain crash after commit
-- enters reconciliation; it never replays the consumed command/lease.

ALTER TABLE local_operation_journal
    ADD COLUMN revocation_epoch INTEGER NOT NULL DEFAULT 0 CHECK (revocation_epoch >= 0);
ALTER TABLE local_operation_journal
    ADD COLUMN purpose TEXT NOT NULL DEFAULT 'unspecified';
ALTER TABLE local_operation_journal
    ADD COLUMN originating_task_id TEXT;
ALTER TABLE local_operation_journal
    ADD COLUMN arguments_schema_uri TEXT NOT NULL DEFAULT 'unspecified';
ALTER TABLE local_operation_journal
    ADD COLUMN expected_resource_revision TEXT;
ALTER TABLE local_operation_journal
    ADD COLUMN resource_observed_at TEXT;
ALTER TABLE local_operation_journal
    ADD COLUMN resource_owning_device_id TEXT;

CREATE UNIQUE INDEX local_operation_account_device_operation_uq
    ON local_operation_journal (account_id, operation_id, target_device_id);

CREATE TABLE local_offline_lease (
    lease_id TEXT PRIMARY KEY,
    issuer_id TEXT NOT NULL,
    audience TEXT NOT NULL,
    operation_id TEXT NOT NULL UNIQUE,
    account_id TEXT NOT NULL,
    target_device_id TEXT NOT NULL,
    target_credential_generation INTEGER NOT NULL CHECK (target_credential_generation > 0),
    capability_id TEXT NOT NULL,
    capability_schema_version INTEGER NOT NULL CHECK (capability_schema_version > 0),
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    resource_generation TEXT NOT NULL,
    resource_observed_revision TEXT NOT NULL,
    resource_observed_at TEXT NOT NULL,
    resource_owning_device_id TEXT NOT NULL,
    arguments_schema_uri TEXT NOT NULL,
    arguments_sha256 BLOB NOT NULL CHECK (length(arguments_sha256) = 32),
    purpose TEXT NOT NULL,
    originating_task_id TEXT,
    grant_reference TEXT NOT NULL,
    policy_version INTEGER NOT NULL CHECK (policy_version > 0),
    revocation_epoch INTEGER NOT NULL CHECK (revocation_epoch >= 0),
    risk_tier INTEGER NOT NULL CHECK (risk_tier BETWEEN 0 AND 1),
    nonce TEXT NOT NULL,
    signing_key_generation INTEGER NOT NULL CHECK (signing_key_generation > 0),
    issued_at TEXT NOT NULL,
    not_before TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    received_at TEXT NOT NULL,
    lease_state TEXT NOT NULL CHECK (lease_state IN ('stored', 'consumed', 'rejected', 'expired_unused')),
    consumed_at TEXT,
    reason_code TEXT,
    CHECK (resource_owning_device_id = target_device_id),
    CHECK (julianday(not_before) IS NOT NULL AND julianday(issued_at) IS NOT NULL AND julianday(not_before) >= julianday(issued_at)),
    CHECK (julianday(expires_at) IS NOT NULL AND julianday(expires_at) > julianday(not_before)),
    CHECK (julianday(expires_at) <= julianday(issued_at) + 5.0/1440.0),
    CHECK ((lease_state = 'consumed') = (consumed_at IS NOT NULL)),
    FOREIGN KEY (account_id, operation_id, target_device_id)
        REFERENCES local_operation_journal (account_id, operation_id, target_device_id)
);

CREATE INDEX local_offline_lease_open_idx
    ON local_offline_lease (account_id, expires_at)
    WHERE lease_state = 'stored';
CREATE UNIQUE INDEX local_offline_lease_nonce_uq
    ON local_offline_lease (account_id, nonce);

CREATE TRIGGER local_offline_lease_matches_operation_insert
BEFORE INSERT ON local_offline_lease
WHEN NOT EXISTS (
    SELECT 1 FROM local_operation_journal o
     WHERE o.account_id = NEW.account_id
       AND o.operation_id = NEW.operation_id
       AND o.target_device_id = NEW.target_device_id
       AND o.target_credential_generation = NEW.target_credential_generation
       AND o.capability_id = NEW.capability_id
       AND o.capability_schema_version = NEW.capability_schema_version
       AND o.resource_type = NEW.resource_type
       AND o.resource_id = NEW.resource_id
       AND o.resource_generation = NEW.resource_generation
       AND o.expected_resource_revision = NEW.resource_observed_revision
       AND o.resource_observed_at = NEW.resource_observed_at
       AND o.resource_owning_device_id = NEW.resource_owning_device_id
       AND o.arguments_schema_uri = NEW.arguments_schema_uri
       AND o.arguments_sha256 = NEW.arguments_sha256
       AND o.grant_reference = NEW.grant_reference
       AND o.policy_version = NEW.policy_version
       AND o.revocation_epoch = NEW.revocation_epoch
       AND o.risk_tier = NEW.risk_tier
       AND o.purpose = NEW.purpose
       AND o.originating_task_id IS NEW.originating_task_id
)
BEGIN
    SELECT RAISE(ABORT, 'offline lease claims do not match the referenced operation');
END;

CREATE TRIGGER local_operation_claims_immutable_after_lease
BEFORE UPDATE OF account_id, operation_id, target_device_id, target_credential_generation,
    capability_id, capability_schema_version, resource_type, resource_id,
    resource_generation, expected_resource_revision, resource_observed_at,
    resource_owning_device_id, arguments_schema_uri, arguments_sha256,
    grant_reference, policy_version, revocation_epoch, risk_tier, purpose, originating_task_id
ON local_operation_journal
WHEN EXISTS (
    SELECT 1 FROM local_offline_lease l
     WHERE l.account_id = OLD.account_id AND l.operation_id = OLD.operation_id
)
BEGIN
    SELECT RAISE(ABORT, 'operation claims are immutable after an offline lease is stored');
END;

CREATE TRIGGER local_offline_lease_immutable_claims
BEFORE UPDATE OF lease_id, issuer_id, audience, operation_id, account_id, target_device_id,
    target_credential_generation, capability_id, capability_schema_version,
    resource_type, resource_id, resource_generation, resource_observed_revision,
    resource_observed_at, resource_owning_device_id, arguments_schema_uri,
    arguments_sha256, purpose, originating_task_id, grant_reference, policy_version,
    revocation_epoch, risk_tier, nonce, signing_key_generation, issued_at, not_before, expires_at
ON local_offline_lease
BEGIN
    SELECT RAISE(ABORT, 'offline lease claims are immutable');
END;

-- Set nonce.expires_at to signed-message expiry PLUS the configured maximum accepted clock skew.
-- Nonce claims are immutable and cannot be deleted before that retention deadline.
-- Rows are retained through restore/rollback and are never reset by an app downgrade.
CREATE TABLE local_consumed_nonce (
    account_id TEXT NOT NULL,
    message_type TEXT NOT NULL CHECK (message_type IN ('command', 'offline_lease')),
    nonce TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    consumed_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    PRIMARY KEY (account_id, message_type, nonce),
    UNIQUE (account_id, message_type, reference_id)
);

CREATE TRIGGER local_offline_lease_must_start_stored
BEFORE INSERT ON local_offline_lease
WHEN NEW.lease_state <> 'stored' OR NEW.consumed_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'new offline lease must start in stored state');
END;

CREATE TRIGGER local_offline_lease_consumption_requires_nonce
BEFORE UPDATE OF lease_state ON local_offline_lease
WHEN NEW.lease_state = 'consumed'
 AND NOT EXISTS (
     SELECT 1 FROM local_consumed_nonce n
      WHERE n.account_id = NEW.account_id
        AND n.message_type = 'offline_lease'
        AND n.nonce = NEW.nonce
        AND n.reference_id = NEW.lease_id
        AND julianday(n.expires_at) IS NOT NULL
        AND julianday(n.expires_at) >= julianday(NEW.expires_at)
 )
BEGIN
    SELECT RAISE(ABORT, 'consume nonce must commit before lease state advances');
END;

CREATE TRIGGER local_offline_lease_consumption_requires_operation_marker
BEFORE UPDATE OF lease_state ON local_offline_lease
WHEN NEW.lease_state = 'consumed'
 AND NOT EXISTS (
     SELECT 1 FROM local_operation_journal o
      WHERE o.account_id = NEW.account_id
        AND o.operation_id = NEW.operation_id
        AND o.target_device_id = NEW.target_device_id
        AND o.side_effect_may_have_occurred = 1
        AND o.current_status IN ('running', 'reconciliation_required', 'unknown_outcome')
 )
BEGIN
    SELECT RAISE(ABORT, 'operation must be durably marked as possibly effected before lease consumption');
END;

-- Highest accepted CommandEnvelope.session_sequence per authenticated control session.
-- Insert this row at 0 when OpenSession succeeds. Set retain_until to session expiry plus the
-- configured maximum accepted clock skew. A new authenticated session gets a new session_id.
CREATE TABLE local_session_sequence (
    session_id TEXT PRIMARY KEY,
    last_session_sequence INTEGER NOT NULL CHECK (last_session_sequence >= 0),
    updated_at TEXT NOT NULL,
    retain_until TEXT NOT NULL
        CHECK (julianday(retain_until) IS NOT NULL AND julianday(retain_until) > julianday(updated_at))
);

-- One BEGIN IMMEDIATE transaction inserts nonce, marks the operation as possibly effected,
-- advances the lease to consumed, and commits before the adapter executes. A missing response
-- after commit requires reconciliation, never blind replay.

CREATE TRIGGER local_consumed_nonce_immutable
BEFORE UPDATE ON local_consumed_nonce
BEGIN
    SELECT RAISE(ABORT, 'consumed nonce records are immutable');
END;

CREATE TRIGGER local_consumed_nonce_retention
BEFORE DELETE ON local_consumed_nonce
WHEN julianday(OLD.expires_at) IS NULL OR julianday(OLD.expires_at) > julianday('now')
BEGIN
    SELECT RAISE(ABORT, 'consumed nonce cannot be deleted before its replay-retention deadline');
END;

CREATE TRIGGER local_session_sequence_initial_zero
BEFORE INSERT ON local_session_sequence
WHEN NEW.last_session_sequence <> 0
BEGIN
    SELECT RAISE(ABORT, 'new session sequence watermark must start at zero');
END;

CREATE TRIGGER local_session_sequence_monotonic
BEFORE UPDATE ON local_session_sequence
WHEN NEW.session_id IS NOT OLD.session_id
  OR NEW.last_session_sequence <= OLD.last_session_sequence
  OR NEW.retain_until IS NOT OLD.retain_until
BEGIN
    SELECT RAISE(ABORT, 'session sequence must strictly increase; identity and retention deadline are immutable');
END;

CREATE TRIGGER local_session_sequence_retention
BEFORE DELETE ON local_session_sequence
WHEN julianday(OLD.retain_until) IS NULL
  OR julianday(OLD.retain_until) > julianday('now')
BEGIN
    SELECT RAISE(ABORT, 'session sequence watermark cannot be deleted before expiry plus clock-skew retention');
END;
