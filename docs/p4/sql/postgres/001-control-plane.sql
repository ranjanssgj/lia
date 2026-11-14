-- LIA P4 Draft migration 001: device/control metadata and durable delivery records.
-- PostgreSQL 16. This migration intentionally does not create memory, grants,
-- Temporal internal tables, or canonical run/step tables.
-- Execute through the migration runner in one transaction. Temporal persistence
-- uses a separate database/schema/role boundary and its own migrations.

CREATE SCHEMA lia_control;

CREATE TABLE lia_control.device_registry (
    device_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    device_kind TEXT NOT NULL CHECK (device_kind IN ('linux_desktop', 'windows_desktop', 'android', 'ios', 'home_gateway', 'service_worker')),
    display_name TEXT,
    platform_id TEXT NOT NULL,
    platform_version TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL CHECK (lifecycle_state IN ('unenrolled', 'pairing_pending', 'enrolled_not_ready', 'ready', 'degraded', 'suspended', 'revoked', 'updating', 'incompatible', 'removed')),
    credential_generation BIGINT NOT NULL CHECK (credential_generation > 0),
    revocation_epoch BIGINT NOT NULL DEFAULT 0 CHECK (revocation_epoch >= 0),
    protocol_version TEXT NOT NULL,
    enrolled_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK ((lifecycle_state <> 'revoked' OR revoked_at IS NOT NULL) AND (revoked_at IS NULL OR lifecycle_state IN ('revoked', 'removed')))
);

CREATE INDEX device_registry_account_state_idx
    ON lia_control.device_registry (account_id, lifecycle_state, last_seen_at DESC);

CREATE TABLE lia_control.capability_manifest_snapshot (
    device_id UUID NOT NULL REFERENCES lia_control.device_registry(device_id),
    account_id UUID NOT NULL,
    manifest_revision BIGINT NOT NULL CHECK (manifest_revision > 0),
    contract_version TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    adapter_version TEXT NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    manifest_digest_sha256 BYTEA NOT NULL CHECK (octet_length(manifest_digest_sha256) = 32),
    manifest JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (device_id, manifest_revision),
    CHECK (valid_until > observed_at)
);

CREATE INDEX capability_manifest_current_idx
    ON lia_control.capability_manifest_snapshot (account_id, device_id, valid_until DESC);

CREATE TABLE lia_control.operation_journal (
    operation_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    target_device_id UUID NOT NULL REFERENCES lia_control.device_registry(device_id),
    target_credential_generation BIGINT NOT NULL CHECK (target_credential_generation > 0),
    capability_id TEXT NOT NULL,
    capability_schema_version INTEGER NOT NULL CHECK (capability_schema_version > 0),
    action_class TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    resource_generation TEXT NOT NULL,
    expected_resource_revision TEXT,
    arguments_schema_uri TEXT NOT NULL,
    arguments_sha256 BYTEA NOT NULL CHECK (octet_length(arguments_sha256) = 32),
    grant_reference UUID NOT NULL,
    policy_version BIGINT NOT NULL CHECK (policy_version > 0),
    risk_tier SMALLINT NOT NULL CHECK (risk_tier BETWEEN 0 AND 4),
    confirmation_reference UUID,
    purpose TEXT NOT NULL,
    objective_reference UUID,
    campaign_reference UUID,
    run_reference UUID,
    step_reference UUID,
    current_status TEXT NOT NULL CHECK (current_status IN ('proposed', 'authorized', 'dispatched', 'acknowledged', 'running', 'verifying', 'reconciliation_required', 'succeeded_verified', 'failed', 'unknown_outcome', 'cancelled', 'rejected', 'expired')),
    correlation_id UUID NOT NULL,
    causation_id UUID,
    issued_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (expires_at IS NULL OR issued_at IS NULL OR expires_at > issued_at)
);

CREATE INDEX operation_journal_account_status_idx
    ON lia_control.operation_journal (account_id, current_status, created_at DESC);
CREATE INDEX operation_journal_target_idx
    ON lia_control.operation_journal (target_device_id, current_status, created_at DESC);

CREATE TABLE lia_control.operation_attempt (
    attempt_id UUID PRIMARY KEY,
    command_id UUID NOT NULL UNIQUE,
    operation_id UUID NOT NULL REFERENCES lia_control.operation_journal(operation_id),
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    worker_lease_generation BIGINT NOT NULL CHECK (worker_lease_generation > 0),
    attempt_status TEXT NOT NULL CHECK (attempt_status IN ('created', 'dispatched', 'acknowledged', 'running', 'completed', 'failed', 'cancelled', 'outcome_unknown', 'reconciliation_required')),
    dispatched_at TIMESTAMPTZ,
    acknowledged_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    side_effect_may_have_occurred BOOLEAN NOT NULL DEFAULT FALSE,
    failure_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (operation_id, attempt_number),
    UNIQUE (operation_id, worker_lease_generation)
);

CREATE TABLE lia_control.operation_evidence (
    evidence_id UUID PRIMARY KEY,
    operation_id UUID NOT NULL REFERENCES lia_control.operation_journal(operation_id),
    attempt_id UUID REFERENCES lia_control.operation_attempt(attempt_id),
    account_id UUID NOT NULL,
    evidence_type TEXT NOT NULL,
    verification_method TEXT NOT NULL,
    source_device_id UUID REFERENCES lia_control.device_registry(device_id),
    source_adapter_id TEXT NOT NULL,
    source_adapter_version TEXT NOT NULL,
    resource_revision_before TEXT,
    resource_revision_after TEXT,
    observed_at TIMESTAMPTZ NOT NULL,
    evidence_reference TEXT,
    evidence_digest_sha256 BYTEA CHECK (evidence_digest_sha256 IS NULL OR octet_length(evidence_digest_sha256) = 32),
    independently_verified BOOLEAN NOT NULL DEFAULT FALSE,
    verification_status TEXT NOT NULL CHECK (verification_status IN ('pending', 'verified', 'rejected', 'inconclusive')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX operation_evidence_operation_idx
    ON lia_control.operation_evidence (account_id, operation_id, observed_at);

CREATE TABLE lia_control.event_outbox (
    event_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    aggregate_sequence BIGINT NOT NULL CHECK (aggregate_sequence > 0),
    producer_id TEXT NOT NULL,
    source_event_id TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    correlation_id UUID NOT NULL,
    causation_id UUID,
    payload JSONB,
    payload_reference TEXT,
    payload_digest_sha256 BYTEA NOT NULL CHECK (octet_length(payload_digest_sha256) = 32),
    sensitivity SMALLINT NOT NULL CHECK (sensitivity BETWEEN 0 AND 3),
    published_at TIMESTAMPTZ,
    publish_attempts INTEGER NOT NULL DEFAULT 0 CHECK (publish_attempts >= 0),
    CHECK ((payload IS NOT NULL) <> (payload_reference IS NOT NULL)),
    UNIQUE (account_id, aggregate_type, aggregate_id, aggregate_sequence)
);

CREATE INDEX event_outbox_pending_idx
    ON lia_control.event_outbox (recorded_at)
    WHERE published_at IS NULL;

CREATE TABLE lia_control.event_inbox (
    consumer_id TEXT NOT NULL,
    event_id UUID NOT NULL,
    account_id UUID NOT NULL,
    event_digest_sha256 BYTEA NOT NULL CHECK (octet_length(event_digest_sha256) = 32),
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    aggregate_sequence BIGINT NOT NULL CHECK (aggregate_sequence > 0),
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    disposition TEXT NOT NULL CHECK (disposition IN ('received', 'processed', 'duplicate', 'deferred_gap', 'quarantined', 'rejected')),
    reason_code TEXT,
    PRIMARY KEY (consumer_id, event_id)
);

CREATE INDEX event_inbox_unprocessed_idx
    ON lia_control.event_inbox (consumer_id, received_at)
    WHERE processed_at IS NULL;

-- Ownership notes:
-- * Permission grants/approvals are referenced by ID; they are not copied here.
-- * Temporal owns durable workflow execution histories. Any HTTP/query projection
--   of runs/steps is derived and cannot drive side effects.
-- * Memory records and retrieval indexes belong to the governed Memory Authority.
-- * Provider-owned state remains provider-authoritative.
-- * Service queries must always bind account_id and validate object-level grants.
-- * Do not place private keys, connector credentials, raw capture streams, full
--   prompts, or unrestricted memory content in these tables.
-- * Migration rollback must preserve revocation epochs, tombstones and audit history.
