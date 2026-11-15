import { readFile } from "node:fs/promises";
import sqlite3InitModule from "@sqlite.org/sqlite-wasm";

const sqlite3 = await sqlite3InitModule();
const db = new sqlite3.oo1.DB(":memory:");
try {
  db.exec("PRAGMA foreign_keys = ON; PRAGMA recursive_triggers = ON;");
  for (const migration of [
    "docs/p4/sql/sqlite/001-device-agent.sql",
    "docs/p4/sql/sqlite/002-retention-tombstones.sql",
    "docs/p4/sql/sqlite/003-consumption-ledger.sql",
    "docs/p4/sql/sqlite/004-manifest-revision-json-safe.sql",
    "docs/p4/sql/sqlite/005-sequence-zero-initial-use.sql",
  ]) {
    db.exec(await readFile(migration, "utf8"));
    console.log(`applied: ${migration}`);
  }
  const [{ version }] = db.exec({
    sql: "SELECT sqlite_version() AS version",
    returnValue: "resultRows",
    rowMode: "object",
  });
  console.log(`SQLite runtime: ${version}`);
  if (version !== "3.54.1") throw new Error(`Expected SQLite 3.54.1, got ${version}`);

  const insertSession = `INSERT INTO local_session_sequence
    (session_id, last_session_sequence, updated_at, retain_until)
    VALUES ('sequence-zero', 0, '2026-11-15T00:00:00Z', '2099-01-01T00:00:00Z')`;
  db.exec(insertSession);
  db.exec(`UPDATE local_session_sequence
    SET last_session_sequence = 0, has_accepted_sequence = 1,
        updated_at = '2026-11-15T00:00:01Z'
    WHERE session_id = 'sequence-zero'`);

  const mustReject = (sql, label) => {
    try {
      db.exec(sql);
    } catch {
      console.log(`rejected as required: ${label}`);
      return;
    }
    throw new Error(`Expected rejection: ${label}`);
  };
  mustReject(`UPDATE local_session_sequence SET last_session_sequence = 0,
    updated_at = '2026-11-15T00:00:02Z' WHERE session_id = 'sequence-zero'`, "duplicate sequence zero");
  mustReject(`UPDATE local_session_sequence SET last_session_sequence = 1,
    has_accepted_sequence = 0, updated_at = '2026-11-15T00:00:02Z'
    WHERE session_id = 'sequence-zero'`, "clearing accepted-sequence marker");
  db.exec(`UPDATE local_session_sequence SET last_session_sequence = 1,
    updated_at = '2026-11-15T00:00:02Z' WHERE session_id = 'sequence-zero'`);
  mustReject(`UPDATE local_session_sequence SET last_session_sequence = 0,
    updated_at = '2026-11-15T00:00:03Z' WHERE session_id = 'sequence-zero'`, "backward sequence update");
  mustReject(`DELETE FROM local_session_sequence WHERE session_id = 'sequence-zero'`, "premature watermark deletion");
} finally {
  db.close();
}
