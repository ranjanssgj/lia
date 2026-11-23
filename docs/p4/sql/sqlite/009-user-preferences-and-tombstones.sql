-- LIA P4 Draft SQLite migration 009: local user preferences and provenance.
-- Supports AC-009: one permitted low-sensitivity preference persists with provenance,
-- inspectable and deletable through user controls, bound to local deletion tombstones.

CREATE TABLE local_user_preference (
    preference_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    preference_key TEXT NOT NULL,
    preference_value TEXT NOT NULL,
    sensitivity TEXT NOT NULL CHECK (sensitivity IN ('S0', 'S1')),
    scope_type TEXT NOT NULL CHECK (scope_type IN ('account', 'device', 'session')),
    scope_reference TEXT NOT NULL,
    source_reference_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    created_at_us INTEGER NOT NULL,
    updated_at_us INTEGER NOT NULL,
    UNIQUE (account_id, preference_key)
);

CREATE INDEX local_user_preference_active_idx
    ON local_user_preference (account_id, preference_key)
    WHERE is_deleted = 0;
