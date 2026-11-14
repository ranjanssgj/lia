-- LIA P4 draft SQLite migration 004: enforce the interoperable manifest revision range.
-- JSON/JCS and common JavaScript consumers represent integers exactly only through 2^53 - 1.
CREATE TRIGGER capability_manifest_revision_json_safe_insert
BEFORE INSERT ON capability_manifest_cache
WHEN NEW.manifest_revision < 1 OR NEW.manifest_revision > 9007199254740991
BEGIN
    SELECT RAISE(ABORT, 'manifest_revision is outside the JSON-safe integer range');
END;

CREATE TRIGGER capability_manifest_revision_json_safe_update
BEFORE UPDATE OF manifest_revision ON capability_manifest_cache
WHEN NEW.manifest_revision < 1 OR NEW.manifest_revision > 9007199254740991
BEGIN
    SELECT RAISE(ABORT, 'manifest_revision is outside the JSON-safe integer range');
END;
