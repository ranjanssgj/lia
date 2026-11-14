-- LIA P4 Draft migration 001: one device installation's local journal and cache.
-- SQLite WAL is for a single host only. Do not place this database on a network filesystem.
-- Enable WAL once during database bootstrap, outside a transaction:
--   PRAGMA journal_mode = WAL;
-- Every connection must enable:
--   PRAGMA foreign_keys = ON;
-- The busy timeout and checkpoint policy are runtime settings and must be benchmarked.

CREATE TABLE local_operation_journal (
    operation_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    target_device_id TEXT NOT NULL,
    target_credential_generation INTEGER NOT NULL CHECK (target_credential_generation > 0),
    capability_id TEXT NOT NULL,
    capability_schema_version INTEGER NOT NULL CHECK (capability_schema_version > 0),
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    resource_generation TEXT NOT NULL,
    arguments_sha256 BLOB NOT NULL CHECK (length(arguments_sha256) = 32),
    grant_reference TEXT NOT NULL,
    policy_version INTEGER NOT NULL,
    risk_tier INTEGER NOT NULL CHECK (risk_tier BETWEEN 0 AND 4),
    current_status TEXT NOT NULL CHECK (current_status IN ('received', 'accepted', 'rejected', 'running', 'verifying', 'reconciliation_required', 'succeeded_verified', 'failed', 'unknown_outcome', 'cancelled', 'expired')),
    side_effect_may_have_occurred INTEGER NOT NULL DEFAULT 0 CHECK (side_effect_may_have_occurred IN (0, 1)),
    received_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE local_operation_attempt (
    attempt_id TEXT PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    operation_id TEXT NOT NULL REFERENCES local_operation_journal(operation_id),
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    worker_lease_generation INTEGER NOT NULL CHECK (worker_lease_generation > 0),
    attempt_status TEXT NOT NULL CHECK (attempt_status IN ('created', 'started', 'completed', 'failed', 'cancelled', 'outcome_unknown', 'reconciliation_required')),
    started_at TEXT,
    completed_at TEXT,
    failure_code TEXT,
    UNIQUE (operation_id, attempt_number),
    UNIQUE (operation_id, worker_lease_generation)
);

CREATE TABLE local_outbox (
    event_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    producer_id TEXT NOT NULL,
    source_event_id TEXT,
    event_type TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    aggregate_sequence INTEGER NOT NULL CHECK (aggregate_sequence > 0),
    correlation_id TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    payload_json TEXT,
    payload_reference TEXT,
    payload_digest_sha256 BLOB NOT NULL CHECK (length(payload_digest_sha256) = 32),
    sensitivity INTEGER NOT NULL CHECK (sensitivity BETWEEN 0 AND 3),
    expires_at TEXT NOT NULL,
    delivery_state TEXT NOT NULL CHECK (delivery_state IN ('pending', 'accepted', 'quarantined', 'expired')),
    delivery_attempts INTEGER NOT NULL DEFAULT 0 CHECK (delivery_attempts >= 0),
    UNIQUE (account_id, aggregate_type, aggregate_id, aggregate_sequence),
    CHECK ((payload_json IS NOT NULL) <> (payload_reference IS NOT NULL))
);

CREATE INDEX local_outbox_pending_idx
    ON local_outbox (delivery_state, occurred_at)
    WHERE delivery_state = 'pending';

CREATE UNIQUE INDEX local_outbox_source_identity_idx
    ON local_outbox (account_id, producer_id, source_event_id)
    WHERE source_event_id IS NOT NULL;

CREATE TABLE capability_manifest_cache (
    device_id TEXT NOT NULL,
    manifest_revision INTEGER NOT NULL CHECK (manifest_revision > 0),
    contract_version TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    manifest_digest_sha256 BLOB NOT NULL CHECK (length(manifest_digest_sha256) = 32),
    manifest_json TEXT NOT NULL,
    PRIMARY KEY (device_id, manifest_revision),
    CHECK (valid_until > observed_at)
);

-- Only bounded, authorized metadata belongs in this cache. The database must not
-- become a general screen/audio/camera archive or canonical user-memory store.
-- S2 content requires its own explicit task/retention authorization; S3 secrets,
-- private keys and connector credentials are prohibited here.
-- A local outbox row is not globally accepted until the canonical owner acknowledges
-- the exact event/revision. Reconnect starts with trust/revocation refresh and then
-- replays stable event IDs for deduplication/reconciliation.
