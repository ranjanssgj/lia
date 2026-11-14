-- LIA P4 draft PostgreSQL migration 004: API read projections and exact JSON integer range.
-- Apply after 001-003. These tables are rebuildable projections, never domain authorities.

ALTER TABLE lia_control.capability_manifest_snapshot
    ADD CONSTRAINT capability_manifest_revision_json_safe_ck
    CHECK (manifest_revision BETWEEN 1 AND 9007199254740991);

ALTER TABLE lia_control.device_registry
    ADD CONSTRAINT device_credential_generation_json_safe_ck
        CHECK (credential_generation BETWEEN 1 AND 9007199254740991),
    ADD CONSTRAINT device_revocation_epoch_json_safe_ck
        CHECK (revocation_epoch BETWEEN 0 AND 9007199254740991);

ALTER TABLE lia_control.operation_journal
    ADD CONSTRAINT operation_credential_generation_json_safe_ck
        CHECK (target_credential_generation BETWEEN 1 AND 9007199254740991),
    ADD CONSTRAINT operation_policy_version_json_safe_ck
        CHECK (policy_version BETWEEN 1 AND 9007199254740991),
    ADD CONSTRAINT operation_revocation_epoch_json_safe_ck
        CHECK (revocation_epoch BETWEEN 0 AND 9007199254740991);

CREATE TABLE lia_control.objective_summary_projection (
    account_id UUID NOT NULL,
    objective_id UUID NOT NULL,
    state TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    revision BIGINT NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991),
    active_run_id UUID,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, objective_id),
    CHECK (updated_at >= created_at)
);

CREATE TABLE lia_control.run_summary_projection (
    account_id UUID NOT NULL,
    run_id UUID NOT NULL,
    objective_id UUID NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('queued', 'planning', 'ready', 'running', 'waiting', 'needs_approval', 'reconciliation_required', 'completed', 'completed_with_warnings', 'skipped', 'failed', 'cancelled', 'expired')),
    revision BIGINT NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    reason_code TEXT,
    history_cursor TEXT,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, run_id),
    UNIQUE (account_id, objective_id, run_id),
    FOREIGN KEY (account_id, objective_id)
        REFERENCES lia_control.objective_summary_projection (account_id, objective_id),
    CHECK (updated_at >= created_at)
);

ALTER TABLE lia_control.objective_summary_projection
    ADD CONSTRAINT objective_active_run_same_account_fk
    FOREIGN KEY (account_id, objective_id, active_run_id)
    REFERENCES lia_control.run_summary_projection (account_id, objective_id, run_id)
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE lia_control.grant_summary_projection (
    account_id UUID NOT NULL,
    grant_id UUID NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'expired', 'revoked', 'suspended', 'pending')),
    policy_version BIGINT NOT NULL CHECK (policy_version BETWEEN 1 AND 9007199254740991),
    authority_revision BIGINT NOT NULL CHECK (authority_revision BETWEEN 1 AND 9007199254740991),
    scope_summary TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    target_device_id UUID,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, grant_id),
    FOREIGN KEY (account_id, target_device_id)
        REFERENCES lia_control.device_registry (account_id, device_id)
);

CREATE TABLE lia_control.request_status_projection (
    account_id UUID NOT NULL,
    request_id UUID NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('accepted', 'in_progress', 'awaiting_approval', 'reconciliation_required', 'completed_verified', 'completed_with_warnings', 'failed', 'rejected', 'cancelled', 'expired')),
    accepted_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    revision BIGINT NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991),
    reason_code TEXT,
    run_id UUID,
    operation_id UUID,
    correlation_id UUID NOT NULL,
    projected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, request_id),
    FOREIGN KEY (account_id, run_id)
        REFERENCES lia_control.run_summary_projection (account_id, run_id),
    FOREIGN KEY (account_id, operation_id)
        REFERENCES lia_control.operation_journal (account_id, operation_id),
    CHECK (updated_at >= accepted_at)
);

CREATE TABLE lia_control.api_idempotency_record (
    account_id UUID NOT NULL,
    principal_id TEXT NOT NULL,
    route_key TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_digest_sha256 BYTEA NOT NULL CHECK (octet_length(request_digest_sha256) = 32),
    request_id UUID NOT NULL,
    response_status SMALLINT NOT NULL CHECK (response_status = 202),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (account_id, principal_id, route_key, idempotency_key),
    UNIQUE (account_id, request_id),
    FOREIGN KEY (account_id, request_id)
        REFERENCES lia_control.request_status_projection (account_id, request_id),
    CHECK (expires_at > created_at)
);

CREATE INDEX objective_projection_account_created_idx
    ON lia_control.objective_summary_projection (account_id, created_at DESC);
CREATE INDEX run_projection_account_objective_created_idx
    ON lia_control.run_summary_projection (account_id, objective_id, created_at DESC);
CREATE INDEX grant_projection_account_state_idx
    ON lia_control.grant_summary_projection (account_id, state, projected_at DESC);
CREATE INDEX request_projection_account_updated_idx
    ON lia_control.request_status_projection (account_id, updated_at DESC);
CREATE INDEX idempotency_expiry_idx
    ON lia_control.api_idempotency_record (expires_at);

COMMENT ON TABLE lia_control.objective_summary_projection IS
    'Rebuildable API read model; Temporal/workflow authority owns objective lifecycle.';
COMMENT ON TABLE lia_control.run_summary_projection IS
    'Rebuildable API read model; Temporal workflow history is canonical execution state.';
COMMENT ON TABLE lia_control.grant_summary_projection IS
    'Non-authoritative metadata projection from Permission Authority; never grants or authorizes.';
COMMENT ON TABLE lia_control.request_status_projection IS
    'Rebuildable accepted-request status projection; never proof of an external effect.';
COMMENT ON TABLE lia_control.api_idempotency_record IS
    'Bounded idempotency-key index for accepted API requests; stores request digest and request reference, not request content.';

-- Writers apply only committed, monotonic source revisions and are idempotent by account + ID.
-- API reads still apply principal/object/scope authorization; account_id filtering alone is insufficient.
