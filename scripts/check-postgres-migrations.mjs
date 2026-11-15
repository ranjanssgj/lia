import { readFile } from "node:fs/promises";
import { Client } from "pg";

if (!process.env.DATABASE_URL) throw new Error("DATABASE_URL is required");
const client = new Client({ connectionString: process.env.DATABASE_URL });
await client.connect();
try {
  const { rows } = await client.query("SELECT current_setting('server_version') AS version");
  console.log(`PostgreSQL runtime: ${rows[0].version}`);
  if (!rows[0].version.startsWith("16.16")) {
    throw new Error(`Expected PostgreSQL 16.16, got ${rows[0].version}`);
  }
  for (const migration of [
    "docs/p4/sql/postgres/001-control-plane.sql",
    "docs/p4/sql/postgres/002-retention-deletion.sql",
    "docs/p4/sql/postgres/003-integrity-and-lease-ledger.sql",
    "docs/p4/sql/postgres/004-api-projections-and-revision-bound.sql",
  ]) {
    await client.query(await readFile(migration, "utf8"));
    console.log(`applied: ${migration}`);
  }
} finally {
  await client.end();
}
