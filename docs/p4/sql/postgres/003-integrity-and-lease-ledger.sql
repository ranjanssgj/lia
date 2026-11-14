-- LIA P4 draft migration 003: account-consistency constraints and offline-lease issuance ledger.
-- Additive after 001/002. PostgreSQL 16. Migration IDs are immutable after implementation.
-- This ledger records coordinator issuance/delivery/reporting; the device-local SQLite ledger
-- is the authority for consume-before-effect replay prevention. Neither ledger grants permission.

ALTER TABLE lia_control.device_registry
    ADD CONSTRAINT device_registry_account_device_uk UNIQUE (account_id, device_id);

ALTER TABLE lia_control.operation_journal
    ADD COLUMN revocation_epoch BIGINT NOT NULL DEFAULT 0 CHECK (revocation_epoch >= 0),
    ADD COLUMN originating_task_id UUID,
    ADD COLUMN resource_observed_at TIMESTAMPTZ,
    ADD COLUMN resource_owning_device_id UUID,
    ADD CONSTRAINT operation_journal_account_operation_uk UNIQUE (account_id, operation_id),
    ADD CONSTRAINT operation_journal_account_device_operation_uk
        UNIQUE (account_id, operation_id, target_device_id),
    ADD CONSTRAINT operation_journal_account_device_fk
        FOREIGN KEY (account_id, target_device_id)
        REFERENCES lia_control.device_registry (account_id, device_id);

ALTER TABLE lia_control.capability_manifest_snapshot
    ADD CONSTRAINT capability_manifest_account_device_fk
        FOREIGN KEY (account_id, device_id)
        REFERENCES lia_control.device_registry (account_id, device_id);

ALTER TABLE lia_control.operation_attempt
    ADD CONSTRAINT operation_attempt_operation_attempt_uk UNIQUE (operation_id, attempt_id);

ALTER TABLE lia_control.operation_evidence
    ADD CONSTRAINT operation_evidence_account_operation_fk
        FOREIGN KEY (account_id, operation_id)
        REFERENCES lia_control.operation_journal (account_id, operation_id),
    ADD CONSTRAINT operation_evidence_account_source_device_fk
        FOREIGN KEY (account_id, source_device_id)
        REFERENCES lia_control.device_registry (account_id, device_id),
    ADD CONSTRAINT operation_evidence_operation_attempt_fk
        FOREIGN KEY (operation_id, attempt_id)
        REFERENCES lia_control.operation_attempt (operation_id, attempt_id);

CREATE TABLE lia_control.offline_lease_issuance (
    lease_id UUID PRIMARY KEY,
    issuer_id TEXT NOT NULL,
    audience TEXT NOT NULL,
    account_id UUID NOT NULL,
    target_device_id UUID NOT NULL,
    target_credential_generation BIGINT NOT NULL CHECK (target_credential_generation > 0),
    operation_id UUID NOT NULL,
    capability_id TEXT NOT NULL,
    capability_schema_version INTEGER NOT NULL CHECK (capability_schema_version > 0),
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    resource_generation TEXT NOT NULL,
    resource_observed_revision TEXT NOT NULL,
    resource_observed_at TIMESTAMPTZ NOT NULL,
    resource_owning_device_id UUID NOT NULL,
    arguments_schema_uri TEXT NOT NULL,
    arguments_sha256 BYTEA NOT NULL CHECK (octet_length(arguments_sha256) = 32),
    purpose TEXT NOT NULL,
    originating_task_id UUID,
    grant_reference UUID NOT NULL,
    policy_version BIGINT NOT NULL CHECK (policy_version > 0),
    revocation_epoch BIGINT NOT NULL CHECK (revocation_epoch >= 0),
    -- SQL 0/1 represent R0/R1; see the normative cross-contract mapping.
    risk_tier SMALLINT NOT NULL CHECK (risk_tier BETWEEN 0 AND 1),
    nonce TEXT NOT NULL,
    signing_key_generation BIGINT NOT NULL CHECK (signing_key_generation > 0),
    issued_at TIMESTAMPTZ NOT NULL,
    not_before TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    lease_state TEXT NOT NULL DEFAULT 'issued'
        CHECK (lease_state IN ('issued', 'delivered', 'reported_consumed', 'expired_unused', 'revoked', 'reconciliation_required')),
    delivery_receipt_at TIMESTAMPTZ,
    consumption_reported_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (not_before >= issued_at),
    CHECK (expires_at > not_before),
    CHECK (expires_at <= issued_at + INTERVAL '5 minutes'),
    CHECK (resource_owning_device_id = target_device_id),
    CHECK (delivery_receipt_at IS NOT NULL OR lease_state <> 'delivered'),
    CHECK ((lease_state = 'reported_consumed') = (consumption_reported_at IS NOT NULL)),
    UNIQUE (operation_id),
    UNIQUE (account_id, target_device_id, nonce),
    FOREIGN KEY (account_id, target_device_id)
        REFERENCES lia_control.device_registry (account_id, device_id),
    FOREIGN KEY (account_id, operation_id, target_device_id)
        REFERENCES lia_control.operation_journal (account_id, operation_id, target_device_id)
);

-- Composite FKs prove account/device/operation identity. This trigger additionally proves
-- that the signed lease claims match the immutable operation claim recorded by the coordinator.
CREATE FUNCTION lia_control.assert_offline_lease_matches_operation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    op lia_control.operation_journal%ROWTYPE;
BEGIN
    SELECT * INTO op
      FROM lia_control.operation_journal
     WHERE account_id = NEW.account_id AND operation_id = NEW.operation_id
     FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'offline lease references an unknown operation';
    END IF;
    IF op.target_device_id IS DISTINCT FROM NEW.target_device_id
       OR op.target_credential_generation IS DISTINCT FROM NEW.target_credential_generation
       OR op.capability_id IS DISTINCT FROM NEW.capability_id
       OR op.capability_schema_version IS DISTINCT FROM NEW.capability_schema_version
       OR op.resource_type IS DISTINCT FROM NEW.resource_type
       OR op.resource_id IS DISTINCT FROM NEW.resource_id
       OR op.resource_generation IS DISTINCT FROM NEW.resource_generation
       OR op.expected_resource_revision IS DISTINCT FROM NEW.resource_observed_revision
       OR op.resource_observed_at IS DISTINCT FROM NEW.resource_observed_at
       OR op.resource_owning_device_id IS DISTINCT FROM NEW.resource_owning_device_id
       OR op.arguments_schema_uri IS DISTINCT FROM NEW.arguments_schema_uri
       OR op.arguments_sha256 IS DISTINCT FROM NEW.arguments_sha256
       OR op.purpose IS DISTINCT FROM NEW.purpose
       OR op.originating_task_id IS DISTINCT FROM NEW.originating_task_id
       OR op.grant_reference IS DISTINCT FROM NEW.grant_reference
       OR op.policy_version IS DISTINCT FROM NEW.policy_version
       OR op.revocation_epoch IS DISTINCT FROM NEW.revocation_epoch
       OR op.risk_tier IS DISTINCT FROM NEW.risk_tier THEN
        RAISE EXCEPTION 'offline lease claims do not match the referenced operation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER offline_lease_matches_operation_trg
    BEFORE INSERT OR UPDATE
    ON lia_control.offline_lease_issuance
    FOR EACH ROW EXECUTE FUNCTION lia_control.assert_offline_lease_matches_operation();

-- Signed lease claims are immutable after issuance; only lifecycle/reporting fields may change.
CREATE FUNCTION lia_control.prevent_offline_lease_claim_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF ROW(NEW.issuer_id, NEW.audience, NEW.account_id, NEW.target_device_id,
           NEW.target_credential_generation, NEW.operation_id, NEW.capability_id,
           NEW.capability_schema_version, NEW.resource_type, NEW.resource_id,
           NEW.resource_generation, NEW.resource_observed_revision, NEW.resource_observed_at,
           NEW.resource_owning_device_id, NEW.arguments_schema_uri, NEW.arguments_sha256,
           NEW.purpose, NEW.originating_task_id, NEW.grant_reference, NEW.policy_version,
           NEW.revocation_epoch, NEW.risk_tier, NEW.nonce, NEW.signing_key_generation,
           NEW.issued_at, NEW.not_before, NEW.expires_at)
       IS DISTINCT FROM
       ROW(OLD.issuer_id, OLD.audience, OLD.account_id, OLD.target_device_id,
           OLD.target_credential_generation, OLD.operation_id, OLD.capability_id,
           OLD.capability_schema_version, OLD.resource_type, OLD.resource_id,
           OLD.resource_generation, OLD.resource_observed_revision, OLD.resource_observed_at,
           OLD.resource_owning_device_id, OLD.arguments_schema_uri, OLD.arguments_sha256,
           OLD.purpose, OLD.originating_task_id, OLD.grant_reference, OLD.policy_version,
           OLD.revocation_epoch, OLD.risk_tier, OLD.nonce, OLD.signing_key_generation,
           OLD.issued_at, OLD.not_before, OLD.expires_at) THEN
        RAISE EXCEPTION 'offline lease signed claims are immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER offline_lease_claims_immutable_trg
    BEFORE UPDATE ON lia_control.offline_lease_issuance
    FOR EACH ROW EXECUTE FUNCTION lia_control.prevent_offline_lease_claim_mutation();

CREATE FUNCTION lia_control.prevent_offline_operation_claim_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM lia_control.offline_lease_issuance l
         WHERE l.account_id = OLD.account_id AND l.operation_id = OLD.operation_id
    ) AND ROW(NEW.target_device_id, NEW.target_credential_generation,
              NEW.capability_id, NEW.capability_schema_version, NEW.resource_type,
              NEW.resource_id, NEW.resource_generation, NEW.expected_resource_revision,
              NEW.resource_observed_at, NEW.resource_owning_device_id, NEW.arguments_schema_uri,
              NEW.arguments_sha256, NEW.purpose, NEW.originating_task_id, NEW.grant_reference,
              NEW.policy_version, NEW.revocation_epoch, NEW.risk_tier)
      IS DISTINCT FROM
          ROW(OLD.target_device_id, OLD.target_credential_generation,
              OLD.capability_id, OLD.capability_schema_version, OLD.resource_type,
              OLD.resource_id, OLD.resource_generation, OLD.expected_resource_revision,
              OLD.resource_observed_at, OLD.resource_owning_device_id, OLD.arguments_schema_uri,
              OLD.arguments_sha256, OLD.purpose, OLD.originating_task_id, OLD.grant_reference,
              OLD.policy_version, OLD.revocation_epoch, OLD.risk_tier) THEN
        RAISE EXCEPTION 'operation claims are immutable after an offline lease is issued';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER operation_claims_immutable_after_lease_trg
    BEFORE UPDATE ON lia_control.operation_journal
    FOR EACH ROW EXECUTE FUNCTION lia_control.prevent_offline_operation_claim_mutation();

CREATE INDEX offline_lease_open_idx
    ON lia_control.offline_lease_issuance (account_id, target_device_id, expires_at)
    WHERE lease_state IN ('issued', 'delivered', 'reconciliation_required');

-- Ledger rows contain identifiers, digests and times only: never signatures, private keys or arguments.
-- A reported consumption is not effect verification; canonical outcome evidence remains separate.
-- Missing reports after offline execution require reconciliation and must not be called unused.
-- Rollback must preserve lease IDs/nonces and revocation epochs.
