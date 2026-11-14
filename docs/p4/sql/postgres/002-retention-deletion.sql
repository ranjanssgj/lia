-- LIA P4 Draft migration 002: retention/deletion orchestration metadata.
-- Canonical tombstones and deletion remain owned by each record domain.
-- These rows track requests, due work and verification references only; no content is stored.
-- Temporal history retention/search attributes remain in the Temporal deployment boundary.

CREATE TABLE lia_control.retention_work_item (
    work_item_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    owner_domain TEXT NOT NULL,
    record_type TEXT NOT NULL,
    record_reference TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('account', 'project', 'task', 'device', 'session', 'external_provider')),
    scope_reference TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('expire', 'archive', 'delete', 'purge_replica', 'purge_index', 'rebuild_projection')),
    purpose TEXT NOT NULL,
    requested_by_principal TEXT NOT NULL,
    policy_version BIGINT,
    expected_record_revision TEXT,
    idempotency_key TEXT NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    not_before TIMESTAMPTZ NOT NULL,
    due_at TIMESTAMPTZ,
    work_state TEXT NOT NULL CHECK (work_state IN ('queued', 'claimed', 'executing', 'awaiting_owner', 'propagating', 'verified', 'blocked', 'failed', 'cancelled')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_reason_code TEXT,
    owner_result_reference TEXT,
    verification_reference TEXT,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (account_id, owner_domain, idempotency_key),
    CHECK (due_at IS NULL OR due_at >= not_before)
);

CREATE INDEX retention_work_due_idx
    ON lia_control.retention_work_item (not_before, due_at, work_state)
    WHERE work_state IN ('queued', 'blocked', 'awaiting_owner', 'propagating');

CREATE INDEX retention_work_owner_idx
    ON lia_control.retention_work_item (account_id, owner_domain, work_state, requested_at DESC);

CREATE TABLE lia_control.retention_work_attempt (
    attempt_id UUID PRIMARY KEY,
    work_item_id UUID NOT NULL REFERENCES lia_control.retention_work_item(work_item_id),
    attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
    executor_id TEXT NOT NULL,
    lease_generation BIGINT NOT NULL CHECK (lease_generation > 0),
    attempt_state TEXT NOT NULL CHECK (attempt_state IN ('started', 'accepted_by_owner', 'verified', 'deferred', 'failed', 'unknown')),
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    owner_response_reference TEXT,
    verification_reference TEXT,
    reason_code TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (work_item_id, attempt_number),
    UNIQUE (work_item_id, lease_generation)
);

CREATE INDEX retention_attempt_work_idx
    ON lia_control.retention_work_attempt (work_item_id, attempt_number DESC);

-- Ownership/security invariants:
-- * The owner_domain service owns the canonical record, tombstone and final delete.
-- * This coordinator table only orchestrates an authorized request and records result references.
-- * A queued work item is not proof that deletion or expiry occurred.
-- * Memory Authority, identity, permission, Temporal, artifact and external-provider owners
--   perform their own tombstone/revision checks and return a verifiable result.
-- * A stale write/replica cannot overwrite a newer tombstone; delete wins over stale update.
-- * Retries reuse idempotency_key and are safe only through owner-domain deduplication.
-- * Work items contain identifiers and metadata only, never record content, secrets or raw capture.
-- * Retention/deletion jobs are account-scoped and re-authorize before every owner request.
-- * Search indexes, caches, derived contexts and replicas are included in propagation/verification.
-- * Backup restoration must re-apply canonical tombstones and current revocation state before access.
