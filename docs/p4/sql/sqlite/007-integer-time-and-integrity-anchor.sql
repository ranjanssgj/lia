-- LIA P4 Draft migration 007: integer timestamps, trusted-time state, and rollback epoch.
-- Versioned timestamps are signed/typed RFC3339 inputs; application code parses them once
-- to exact Unix microseconds. Comparisons and retention guards use only INTEGER values.

ALTER TABLE local_operation_journal ADD COLUMN received_at_us INTEGER;
ALTER TABLE local_operation_journal ADD COLUMN expires_at_us INTEGER;
ALTER TABLE local_operation_journal ADD COLUMN updated_at_us INTEGER;
ALTER TABLE local_operation_attempt ADD COLUMN started_at_us INTEGER;
ALTER TABLE local_operation_attempt ADD COLUMN completed_at_us INTEGER;
ALTER TABLE local_session_sequence ADD COLUMN updated_at_us INTEGER;
ALTER TABLE local_session_sequence ADD COLUMN retain_until_us INTEGER;
ALTER TABLE local_consumed_nonce ADD COLUMN consumed_at_us INTEGER;
ALTER TABLE local_consumed_nonce ADD COLUMN expires_at_us INTEGER;
ALTER TABLE local_offline_lease ADD COLUMN issued_at_us INTEGER;
ALTER TABLE local_offline_lease ADD COLUMN not_before_us INTEGER;
ALTER TABLE local_offline_lease ADD COLUMN expires_at_us INTEGER;
ALTER TABLE local_offline_lease ADD COLUMN received_at_us INTEGER;
ALTER TABLE local_offline_lease ADD COLUMN consumed_at_us INTEGER;
ALTER TABLE local_outbox ADD COLUMN occurred_at_us INTEGER;
ALTER TABLE local_outbox ADD COLUMN expires_at_us INTEGER;
ALTER TABLE local_operation_evidence ADD COLUMN observed_at_us INTEGER;

CREATE TABLE local_integrity_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    anchor_sequence INTEGER NOT NULL CHECK (anchor_sequence >= 0)
);
INSERT INTO local_integrity_state(singleton, anchor_sequence) VALUES (1, 0);

CREATE TABLE local_trusted_clock_state (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    last_trusted_time_us INTEGER NOT NULL CHECK (last_trusted_time_us >= 0)
);
INSERT INTO local_trusted_clock_state(singleton, last_trusted_time_us) VALUES (1, 0);

DROP TRIGGER IF EXISTS local_consumed_nonce_retention;
CREATE TRIGGER local_consumed_nonce_retention
BEFORE DELETE ON local_consumed_nonce
WHEN OLD.expires_at_us IS NULL
  OR OLD.expires_at_us > (SELECT last_trusted_time_us FROM local_trusted_clock_state WHERE singleton = 1)
BEGIN
    SELECT RAISE(ABORT, 'consumed nonce cannot be deleted before trusted integer replay-retention deadline');
END;

DROP TRIGGER IF EXISTS local_session_sequence_retention;
CREATE TRIGGER local_session_sequence_retention
BEFORE DELETE ON local_session_sequence
WHEN OLD.retain_until_us IS NULL
  OR OLD.retain_until_us > (SELECT last_trusted_time_us FROM local_trusted_clock_state WHERE singleton = 1)
BEGIN
    SELECT RAISE(ABORT, 'session sequence watermark cannot be deleted before trusted integer retention deadline');
END;

DROP TRIGGER IF EXISTS local_session_sequence_monotonic;
CREATE TRIGGER local_session_sequence_monotonic
BEFORE UPDATE ON local_session_sequence
WHEN NEW.session_id IS NOT OLD.session_id
  OR NEW.retain_until IS NOT OLD.retain_until
  OR NEW.retain_until_us IS NOT OLD.retain_until_us
  OR NEW.updated_at_us IS NULL
  OR NEW.updated_at_us < OLD.updated_at_us
  OR (OLD.has_accepted_sequence = 0 AND (NEW.has_accepted_sequence <> 1 OR NEW.last_session_sequence <> 0))
  OR (OLD.has_accepted_sequence = 1 AND (NEW.has_accepted_sequence <> 1 OR NEW.last_session_sequence <= OLD.last_session_sequence))
BEGIN
    SELECT RAISE(ABORT, 'first sequence must be zero then strictly increase; trusted integer time and identity/retention are monotonic/immutable');
END;

CREATE TRIGGER local_consumed_nonce_integer_time_required
BEFORE INSERT ON local_consumed_nonce
WHEN NEW.consumed_at_us IS NULL OR NEW.expires_at_us IS NULL OR NEW.expires_at_us < NEW.consumed_at_us
BEGIN
    SELECT RAISE(ABORT, 'consumed nonce requires valid integer timestamps');
END;

CREATE TRIGGER local_offline_lease_integer_time_required
BEFORE INSERT ON local_offline_lease
WHEN NEW.issued_at_us IS NULL OR NEW.not_before_us IS NULL OR NEW.expires_at_us IS NULL
  OR NEW.received_at_us IS NULL OR NEW.not_before_us < NEW.issued_at_us
  OR NEW.expires_at_us <= NEW.not_before_us
  OR NEW.expires_at_us > NEW.issued_at_us + 300000000
BEGIN
    SELECT RAISE(ABORT, 'offline lease integer timestamps are invalid or exceed five minutes');
END;

DROP TRIGGER IF EXISTS local_offline_lease_consumption_requires_nonce;
CREATE TRIGGER local_offline_lease_consumption_requires_nonce
BEFORE UPDATE OF lease_state ON local_offline_lease
WHEN NEW.lease_state = 'consumed'
 AND NOT EXISTS (
     SELECT 1 FROM local_consumed_nonce n
      WHERE n.account_id = NEW.account_id
        AND n.message_type = 'offline_lease'
        AND n.nonce = NEW.nonce
        AND n.reference_id = NEW.lease_id
        AND n.expires_at_us >= NEW.expires_at_us
 )
BEGIN
    SELECT RAISE(ABORT, 'integer consume nonce must commit before lease state advances');
END;

CREATE TRIGGER local_integrity_anchor_monotonic
BEFORE UPDATE ON local_integrity_state
WHEN NEW.singleton IS NOT OLD.singleton OR NEW.anchor_sequence <> OLD.anchor_sequence + 1
BEGIN
    SELECT RAISE(ABORT, 'integrity anchor sequence must advance exactly once');
END;

CREATE TRIGGER local_trusted_clock_monotonic
BEFORE UPDATE ON local_trusted_clock_state
WHEN NEW.singleton IS NOT OLD.singleton OR NEW.last_trusted_time_us < OLD.last_trusted_time_us
BEGIN
    SELECT RAISE(ABORT, 'trusted clock high-water mark cannot move backwards');
END;
