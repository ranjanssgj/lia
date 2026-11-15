# Dependency and model-weight license register

**Status:** Initial inventory; not legal advice or a substitute for artifact-derived SBOM review.
**Last reviewed:** 15 November 2026.

## LIA project license

The owner deferred the project license decision. No `LICENSE` file is added, no open-source or proprietary license is implied by this register, and redistribution terms remain unresolved. Do not publish a release or grant downstream reuse rights until the owner selects the project license and contribution terms.

## Dependency review policy

For dependency triage, apply the policy in the owner's requested decision brief:

- MIT, Apache-2.0, BSD and ISC: eligible for normal review.
- MPL-2.0 and model-weight licenses permitting commercial use: eligible with an explicit register note.
- GPL, AGPL and LGPL: excluded by default; LGPL may be reconsidered only as a separate process with owner approval.
- Research-only and non-commercial terms: excluded by default.
- OpenMDW (Nemotron) and Piper's current `piper1-gpl` repository: require case-by-case review before use.

“Eligible” is not approval. Verify the exact artifact's license, notices, transitive dependencies, redistribution terms and intended use before release. Model card statements and package metadata are leads, not final evidence.

## Locked npm validation dependencies

The table records license strings present in `package-lock.json` at the current main revision. Platform-specific TypeScript packages are listed separately because the lock includes them. The metadata has not yet been reconciled against every package's shipped license files.

| Package | Version | Lock metadata | Review status |
|---|---:|---|---|
| `@redocly/cli` | 2.76.0 | MIT | npm lock metadata; verify before distribution |
| `@sqlite.org/sqlite-wasm` | 3.54.1-build1 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-aix-ppc64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-darwin-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-darwin-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-freebsd-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-freebsd-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-arm` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-loong64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-mips64el` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-ppc64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-riscv64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-s390x` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-linux-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-netbsd-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-netbsd-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-openbsd-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-openbsd-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-sunos-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-win32-arm64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `@typescript/typescript-win32-x64` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `ajv` | 8.20.0 | MIT | npm lock metadata; verify before distribution |
| `ajv-formats` | 3.0.1 | MIT | npm lock metadata; verify before distribution |
| `fast-deep-equal` | 3.1.3 | MIT | npm lock metadata; verify before distribution |
| `fast-uri` | 3.1.9 | BSD-3-Clause | npm lock metadata; verify before distribution |
| `json-schema-traverse` | 1.0.0 | MIT | npm lock metadata; verify before distribution |
| `pg` | 8.23.1 | MIT | npm lock metadata; verify before distribution |
| `pg-cloudflare` | 1.4.1 | MIT | npm lock metadata; verify before distribution |
| `pg-connection-string` | 2.14.1 | MIT | npm lock metadata; verify before distribution |
| `pg-int8` | 1.0.1 | ISC | npm lock metadata; verify before distribution |
| `pg-pool` | 3.14.0 | MIT | npm lock metadata; verify before distribution |
| `pg-protocol` | 1.16.1 | MIT | npm lock metadata; verify before distribution |
| `pg-types` | 2.2.0 | MIT | npm lock metadata; verify before distribution |
| `pgpass` | 1.0.5 | MIT | npm lock metadata; verify before distribution |
| `postgres-array` | 2.0.0 | MIT | npm lock metadata; verify before distribution |
| `postgres-bytea` | 1.0.1 | MIT | npm lock metadata; verify before distribution |
| `postgres-date` | 1.0.7 | MIT | npm lock metadata; verify before distribution |
| `postgres-interval` | 1.2.0 | MIT | npm lock metadata; verify before distribution |
| `require-from-string` | 2.0.2 | MIT | npm lock metadata; verify before distribution |
| `split2` | 4.2.0 | ISC | npm lock metadata; verify before distribution |
| `typescript` | 7.0.2 | Apache-2.0 | npm lock metadata; verify before distribution |
| `xtend` | 4.0.2 | MIT | npm lock metadata; verify before distribution |

## Other pinned tools and runtimes

| Component | Pin | License status |
|---|---|---|
| Rust toolchain | 1.99.1 | Verify distribution notices before bundling |
| Node.js (CI only) | 26.13.1 | Verify distribution notices before bundling |
| npm | 12.2.2 | Verify distribution notices before bundling |
| Protocol Buffers compiler | 36.2 | Verify distribution notices before bundling |
| PostgreSQL container | 16.16 | Verify image/base-layer notices before bundling |
| Godot engine | 4.7.2 | Verify editor, export-template and plugin notices before bundling |
| SQLite WASM | 3.54.1-build1 | See npm lock metadata above; verify shipped notices |

## Model weights and providers

No model weight, LLM, ASR, TTS or embedding package is selected or admitted. Candidate licenses must be reviewed against the intended use, commercial rights, redistribution, attribution, modification, hosting and model-output terms. Do not download or commit weights during M1–M4.

## Release evidence

Before any distributable release, generate an artifact-derived SPDX SBOM, attach exact versions and hashes, reconcile this register with shipped notices, document exceptions, and have the owner review the license and asset-rights findings.
