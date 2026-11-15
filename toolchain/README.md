# Toolchain pins

`versions.toml` is the exact version record for the first P5 Linux slice and its contract CI. The Node pin is only for the CI validator process and does not select Node for the product coordinator. PostgreSQL is used to verify the P4 migration chain, not required by the P5 desktop runtime. The SQLite WASM package is a reproducible migration-check runtime; it does not select the SQLite build for the Rust device agent.

Rust 1.99.1 is pinned because Rust 1.100.0 was released on 12 November 2026 and had not reached a point release by this 14 November review. Godot 4.7.2 remains the blueprint baseline. Hyprland 0.57.2 is a candidate only; an exact compositor baseline remains an owner decision and is intentionally not silently selected.

## Dependency cool-down

New dependencies and version upgrades must be at least seven full days old before entering a lockfile. A shorter window requires an explicit security-fix exception recorded with the affected package, advisory, reviewer and date. Verify transitive lockfile changes, release provenance, license and advisories. A pinned version is not automatically an approved version.

## Primary release sources verified 2026-11-14

- Rust 1.100.0 release and the prior 1.99.1 release: [Rust official release announcements](https://blog.rust-lang.org/releases/).
- Node.js 26.13.1 security release: [Node.js official security release notice](https://nodejs.org/en/blog/vulnerability/november-2026-security-releases/).
- Godot 4.7.2: [Godot 4.7.2 release](https://godotengine.org/article/maintenance-release-godot-4-7-2/).
- Hyprland 0.57.2 candidate: [Hyprland releases](https://github.com/hyprwm/Hyprland/releases).
- PostgreSQL 16.16: [PostgreSQL 16.16 release notes](https://www.postgresql.org/docs/release/16.16/).
- SQLite upstream chronology: [SQLite release chronology](https://www.sqlite.org/chronology.html). CI uses the older, seven-day-cooled-down, exact SQLite 3.54.1 WASM package; the device-side Rust SQLite library remains a separate decision.
- Protobuf 36.2: [Protobuf official releases](https://github.com/protocolbuffers/protobuf/releases).
- TypeScript 7.0.2: [TypeScript official release](https://github.com/microsoft/TypeScript/releases/tag/v7.0.2).
- Redocly CLI 2.76.0, TypeScript 7.0.2, Ajv 8.20.0 and `pg` 8.23.1: exact package metadata in npm lockfile; package publication dates were checked against npm on 2026-11-14. Redocly 2.77.0/2.78.0 were excluded because their seven-day cool-down had not elapsed.

The full distributed coordinator runtime, Temporal, NATS, voice/model vendors, Rust SQLite crate, Hyprland runtime and Godot export templates remain outside this pin set until their respective implementation milestone and owner decisions.
