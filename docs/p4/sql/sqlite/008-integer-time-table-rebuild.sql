-- P4 SQLite migration 008: remove legacy floating-point julianday() CHECK constraints.
-- Rust migration code backfills exact integer microseconds before applying this DDL.

DROP TRIGGER IF EXISTS local_offline_lease_matches_operation_insert;
DROP TRIGGER IF EXISTS local_offline_lease_immutable_claims;
DROP TRIGGER IF EXISTS local_offline_lease_must_start_stored;
DROP TRIGGER IF EXISTS local_offline_lease_consumption_requires_operation_marker;
DROP TRIGGER IF EXISTS local_offline_lease_consumption_requires_nonce;
DROP TRIGGER IF EXISTS local_offline_lease_integer_time_required;
DROP TRIGGER IF EXISTS local_operation_claims_immutable_after_lease;
DROP TRIGGER IF EXISTS local_session_sequence_initial_zero;
DROP TRIGGER IF EXISTS local_session_sequence_monotonic;
DROP TRIGGER IF EXISTS local_session_sequence_retention;
DROP INDEX IF EXISTS local_offline_lease_open_idx;
DROP INDEX IF EXISTS local_offline_lease_nonce_uq;

ALTER TABLE local_offline_lease RENAME TO local_offline_lease_legacy;
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
    issued_at_us INTEGER NOT NULL,
    not_before_us INTEGER NOT NULL,
    expires_at_us INTEGER NOT NULL,
    received_at_us INTEGER NOT NULL,
    consumed_at_us INTEGER,
    CHECK (resource_owning_device_id = target_device_id),
    CHECK (not_before_us >= issued_at_us),
    CHECK (expires_at_us > not_before_us),
    CHECK (expires_at_us <= issued_at_us + 300000000),
    CHECK ((lease_state = 'consumed') = (consumed_at IS NOT NULL)),
    CHECK ((lease_state = 'consumed') = (consumed_at_us IS NOT NULL)),
    FOREIGN KEY (account_id, operation_id, target_device_id)
        REFERENCES local_operation_journal (account_id, operation_id, target_device_id)
);
INSERT INTO local_offline_lease (
    lease_id,issuer_id,audience,operation_id,account_id,target_device_id,target_credential_generation,
    capability_id,capability_schema_version,resource_type,resource_id,resource_generation,
    resource_observed_revision,resource_observed_at,resource_owning_device_id,arguments_schema_uri,
    arguments_sha256,purpose,originating_task_id,grant_reference,policy_version,revocation_epoch,
    risk_tier,nonce,signing_key_generation,issued_at,not_before,expires_at,received_at,lease_state,
    consumed_at,reason_code,issued_at_us,not_before_us,expires_at_us,received_at_us,consumed_at_us
)
SELECT lease_id,issuer_id,audience,operation_id,account_id,target_device_id,target_credential_generation,
    capability_id,capability_schema_version,resource_type,resource_id,resource_generation,
    resource_observed_revision,resource_observed_at,resource_owning_device_id,arguments_schema_uri,
    arguments_sha256,purpose,originating_task_id,grant_reference,policy_version,revocation_epoch,
    risk_tier,nonce,signing_key_generation,issued_at,not_before,expires_at,received_at,lease_state,
    consumed_at,reason_code,issued_at_us,not_before_us,expires_at_us,received_at_us,consumed_at_us
FROM local_offline_lease_legacy;
DROP TABLE local_offline_lease_legacy;

CREATE UNIQUE INDEX local_offline_lease_nonce_uq ON local_offline_lease(account_id, nonce);
CREATE INDEX local_offline_lease_open_idx ON local_offline_lease(account_id, expires_at) WHERE lease_state='stored';

CREATE TRIGGER local_offline_lease_matches_operation_insert
BEFORE INSERT ON local_offline_lease
WHEN NOT EXISTS (
    SELECT 1 FROM local_operation_journal o
     WHERE o.account_id=NEW.account_id AND o.operation_id=NEW.operation_id
       AND o.target_device_id=NEW.target_device_id
       AND o.target_credential_generation=NEW.target_credential_generation
       AND o.capability_id=NEW.capability_id
       AND o.capability_schema_version=NEW.capability_schema_version
       AND o.resource_type=NEW.resource_type AND o.resource_id=NEW.resource_id
       AND o.resource_generation=NEW.resource_generation
       AND o.expected_resource_revision=NEW.resource_observed_revision
       AND o.resource_observed_at=NEW.resource_observed_at
       AND o.resource_owning_device_id=NEW.resource_owning_device_id
       AND o.arguments_schema_uri=NEW.arguments_schema_uri
       AND o.arguments_sha256=NEW.arguments_sha256
       AND o.grant_reference=NEW.grant_reference AND o.policy_version=NEW.policy_version
       AND o.revocation_epoch=NEW.revocation_epoch AND o.risk_tier=NEW.risk_tier
       AND o.purpose=NEW.purpose AND o.originating_task_id IS NEW.originating_task_id
)
BEGIN SELECT RAISE(ABORT, 'offline lease claims do not match the referenced operation'); END;

CREATE TRIGGER local_operation_claims_immutable_after_lease
BEFORE UPDATE OF account_id,operation_id,target_device_id,target_credential_generation,
    capability_id,capability_schema_version,resource_type,resource_id,resource_generation,
    expected_resource_revision,resource_observed_at,resource_owning_device_id,
    arguments_schema_uri,arguments_sha256,grant_reference,policy_version,revocation_epoch,
    risk_tier,purpose,originating_task_id
ON local_operation_journal
WHEN EXISTS (SELECT 1 FROM local_offline_lease l
    WHERE l.account_id=OLD.account_id AND l.operation_id=OLD.operation_id)
BEGIN SELECT RAISE(ABORT, 'operation claims are immutable after an offline lease is stored'); END;

CREATE TRIGGER local_offline_lease_immutable_claims
BEFORE UPDATE OF lease_id,issuer_id,audience,operation_id,account_id,target_device_id,
    target_credential_generation,capability_id,capability_schema_version,resource_type,
    resource_id,resource_generation,resource_observed_revision,resource_observed_at,
    resource_owning_device_id,arguments_schema_uri,arguments_sha256,purpose,originating_task_id,
    grant_reference,policy_version,revocation_epoch,risk_tier,nonce,signing_key_generation,
    issued_at,not_before,expires_at,received_at,issued_at_us,not_before_us,expires_at_us,received_at_us
ON local_offline_lease
BEGIN SELECT RAISE(ABORT, 'offline lease claims are immutable'); END;

CREATE TRIGGER local_offline_lease_must_start_stored
BEFORE INSERT ON local_offline_lease
WHEN NEW.lease_state <> 'stored' OR NEW.consumed_at IS NOT NULL OR NEW.consumed_at_us IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'new offline lease must start in stored state'); END;

CREATE TRIGGER local_offline_lease_consumption_requires_nonce
BEFORE UPDATE OF lease_state ON local_offline_lease
WHEN NEW.lease_state='consumed' AND NOT EXISTS (
    SELECT 1 FROM local_consumed_nonce n WHERE n.account_id=NEW.account_id
      AND n.message_type='offline_lease' AND n.nonce=NEW.nonce
      AND n.reference_id=NEW.lease_id AND n.expires_at_us>=NEW.expires_at_us
)
BEGIN SELECT RAISE(ABORT, 'integer consume nonce must commit before lease state advances'); END;

CREATE TRIGGER local_offline_lease_consumption_requires_operation_marker
BEFORE UPDATE OF lease_state ON local_offline_lease
WHEN NEW.lease_state='consumed' AND NOT EXISTS (
    SELECT 1 FROM local_operation_journal o WHERE o.account_id=NEW.account_id
      AND o.operation_id=NEW.operation_id AND o.target_device_id=NEW.target_device_id
      AND o.side_effect_may_have_occurred=1
      AND o.current_status IN ('running','reconciliation_required','unknown_outcome')
)
BEGIN SELECT RAISE(ABORT, 'operation must be marked possibly effected before lease consumption'); END;

ALTER TABLE local_session_sequence RENAME TO local_session_sequence_legacy;
CREATE TABLE local_session_sequence (
    session_id TEXT PRIMARY KEY,
    last_session_sequence INTEGER NOT NULL CHECK (last_session_sequence >= 0),
    updated_at TEXT NOT NULL,
    retain_until TEXT NOT NULL,
    has_accepted_sequence INTEGER NOT NULL DEFAULT 0 CHECK (has_accepted_sequence IN (0,1)),
    updated_at_us INTEGER NOT NULL,
    retain_until_us INTEGER NOT NULL,
    CHECK (retain_until_us > updated_at_us)
);
INSERT INTO local_session_sequence(session_id,last_session_sequence,updated_at,retain_until,
    has_accepted_sequence,updated_at_us,retain_until_us)
SELECT session_id,last_session_sequence,updated_at,retain_until,has_accepted_sequence,
    updated_at_us,retain_until_us FROM local_session_sequence_legacy;
DROP TABLE local_session_sequence_legacy;

CREATE TRIGGER local_session_sequence_initial_zero
BEFORE INSERT ON local_session_sequence
WHEN NEW.last_session_sequence <> 0
BEGIN SELECT RAISE(ABORT, 'new session sequence watermark must start at zero'); END;
CREATE TRIGGER local_session_sequence_monotonic
BEFORE UPDATE ON local_session_sequence
WHEN NEW.session_id IS NOT OLD.session_id OR NEW.retain_until IS NOT OLD.retain_until
 OR NEW.retain_until_us IS NOT OLD.retain_until_us OR NEW.updated_at_us < OLD.updated_at_us
 OR (OLD.has_accepted_sequence=0 AND (NEW.has_accepted_sequence<>1 OR NEW.last_session_sequence<>0))
 OR (OLD.has_accepted_sequence=1 AND (NEW.has_accepted_sequence<>1 OR NEW.last_session_sequence<=OLD.last_session_sequence))
BEGIN SELECT RAISE(ABORT, 'first sequence must be zero; later values strictly increase'); END;
CREATE TRIGGER local_session_sequence_retention
BEFORE DELETE ON local_session_sequence
WHEN OLD.retain_until_us > (SELECT last_trusted_time_us FROM local_trusted_clock_state WHERE singleton=1)
BEGIN SELECT RAISE(ABORT, 'session watermark cannot be deleted before integer retention deadline'); END;
