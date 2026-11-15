-- LIA P4 Draft migration 006: durable operation outcomes and evidence metadata.
-- Evidence stays local and is never included as raw content in the outbox; only its
-- redacted reference/digest and sensitivity classification are persisted here.

CREATE TABLE local_operation_evidence (
    evidence_id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES local_operation_journal(operation_id),
    attempt_id TEXT NOT NULL REFERENCES local_operation_attempt(attempt_id),
    evidence_kind TEXT NOT NULL,
    observed_outcome TEXT NOT NULL CHECK (observed_outcome IN ('verified_success', 'verified_failure', 'unknown')),
    evidence_sha256 BLOB NOT NULL CHECK (length(evidence_sha256) = 32),
    evidence_reference TEXT NOT NULL,
    sensitivity INTEGER NOT NULL CHECK (sensitivity BETWEEN 0 AND 3),
    observed_at TEXT NOT NULL,
    UNIQUE (attempt_id, evidence_id)
);

CREATE INDEX local_operation_evidence_operation_idx
    ON local_operation_evidence (operation_id, observed_at);

CREATE TRIGGER local_operation_evidence_matches_attempt
BEFORE INSERT ON local_operation_evidence
WHEN NOT EXISTS (
    SELECT 1 FROM local_operation_attempt a
     WHERE a.attempt_id = NEW.attempt_id
       AND a.operation_id = NEW.operation_id
)
BEGIN
    SELECT RAISE(ABORT, 'evidence attempt must belong to the referenced operation');
END;

CREATE TRIGGER local_operation_evidence_immutable
BEFORE UPDATE ON local_operation_evidence
BEGIN
    SELECT RAISE(ABORT, 'operation evidence is immutable');
END;

CREATE TRIGGER local_operation_verified_success_requires_evidence
BEFORE UPDATE OF current_status ON local_operation_journal
WHEN NEW.current_status = 'succeeded_verified'
 AND NOT EXISTS (
    SELECT 1 FROM local_operation_evidence e
    JOIN local_operation_attempt a ON a.attempt_id = e.attempt_id
     WHERE e.operation_id = NEW.operation_id
       AND e.observed_outcome = 'verified_success'
       AND a.attempt_status = 'completed'
 )
BEGIN
    SELECT RAISE(ABORT, 'verified success requires completed attempt evidence');
END;

CREATE TRIGGER local_operation_verified_failure_requires_evidence
BEFORE UPDATE OF current_status ON local_operation_journal
WHEN NEW.current_status = 'failed'
 AND NOT EXISTS (
    SELECT 1 FROM local_operation_evidence e
    JOIN local_operation_attempt a ON a.attempt_id = e.attempt_id
     WHERE e.operation_id = NEW.operation_id
       AND e.observed_outcome = 'verified_failure'
       AND a.attempt_status = 'failed'
 )
BEGIN
    SELECT RAISE(ABORT, 'verified failure requires failed attempt evidence');
END;

CREATE TRIGGER local_operation_unknown_requires_possible_effect
BEFORE UPDATE OF current_status ON local_operation_journal
WHEN NEW.current_status = 'unknown_outcome'
 AND NEW.side_effect_may_have_occurred <> 1
BEGIN
    SELECT RAISE(ABORT, 'unknown outcome requires a durable possible-effect marker');
END;
