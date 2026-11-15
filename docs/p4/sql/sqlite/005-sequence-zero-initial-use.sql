-- LIA P4 Draft SQLite migration 005: allow the first signed session sequence to be zero.
-- Migration 003 initialized the retained watermark at zero and required every value to be
-- strictly greater, which accidentally rejected a first command with session_sequence = 0.
-- The accepted flag distinguishes an unused zero watermark from an already accepted zero.

ALTER TABLE local_session_sequence
    ADD COLUMN has_accepted_sequence INTEGER NOT NULL DEFAULT 0
        CHECK (has_accepted_sequence IN (0, 1));

DROP TRIGGER IF EXISTS local_session_sequence_monotonic;

CREATE TRIGGER local_session_sequence_monotonic
BEFORE UPDATE ON local_session_sequence
WHEN NEW.session_id IS NOT OLD.session_id
  OR NEW.retain_until IS NOT OLD.retain_until
  OR (OLD.has_accepted_sequence = 0 AND (
        NEW.has_accepted_sequence <> 1 OR NEW.last_session_sequence <> 0
  ))
  OR (OLD.has_accepted_sequence = 1 AND (
        NEW.has_accepted_sequence <> 1 OR NEW.last_session_sequence <= OLD.last_session_sequence
  ))
BEGIN
    SELECT RAISE(ABORT, 'first session sequence must be zero; later values must strictly increase with immutable identity and retention');
END;
