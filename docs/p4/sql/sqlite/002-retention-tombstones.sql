-- LIA P4 Draft SQLite migration 002: applied owner-domain tombstones.
-- This is a local replay-prevention/cache marker, not the canonical tombstone.
-- Canonical deletion state is owned by the source domain and reconciled on reconnect.

CREATE TABLE local_deletion_tombstone (
    tombstone_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    owner_domain TEXT NOT NULL,
    record_type TEXT NOT NULL,
    record_reference TEXT NOT NULL,
    scope_type TEXT NOT NULL CHECK (scope_type IN ('account', 'project', 'task', 'device', 'session', 'external_provider')),
    scope_reference TEXT NOT NULL,
    deletion_revision TEXT NOT NULL,
    source_event_id TEXT NOT NULL,
    issued_at TEXT NOT NULL,
    applied_at TEXT,
    apply_state TEXT NOT NULL CHECK (apply_state IN ('pending', 'applied', 'verified', 'conflict', 'blocked')),
    reason_code TEXT,
    UNIQUE (account_id, owner_domain, record_type, record_reference, deletion_revision)
);

CREATE INDEX local_deletion_pending_idx
    ON local_deletion_tombstone (apply_state, issued_at)
    WHERE apply_state IN ('pending', 'conflict', 'blocked');

CREATE TABLE local_retention_cursor (
    owner_domain TEXT PRIMARY KEY,
    last_reconciled_tombstone_id TEXT,
    last_reconciled_at TEXT NOT NULL,
    source_revision TEXT NOT NULL
);

-- Apply tombstones before processing inbound state or restoring cached artifacts.
-- A stale replica/outbox item whose revision predates the owner tombstone cannot
-- recreate or re-expose the deleted record. Conflicts are quarantined for owner sync.
-- No content or secret is stored in these markers.
