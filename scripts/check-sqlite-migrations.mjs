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
} finally {
  db.close();
}
