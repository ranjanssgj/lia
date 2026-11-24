# LIA

LIA is a local-first desktop companion and OS agent. The project is being built around a small Linux desktop slice first: a Godot presentation, a local Rust device agent, a Hyprland adapter, and a conversation path that can request one narrowly scoped window operation. The larger product includes persistent memory, voice, multiple platforms and distributed coordination, but those capabilities are not all present in this slice.

The repository contains both implementation code and versioned design contracts. The contracts under `docs/p4/` are still marked **Draft**. Synthetic tests and contract checks help catch implementation and schema errors; they do not make those contracts verified or establish live desktop support.

## What is here

- **Rust device agent** (`crates/lia-device-agent/`): local SQLite state, replay and nonce protection, signed-message helpers, presentation IPC, Hyprland discovery and operations, conversation coordination, synthetic model and speech providers, and low-sensitivity preference memory.
- **Godot presentation** (`apps/lia-presentation-godot/`): a Godot project that connects to the agent over a framed Unix socket, parses snapshots, tracks presentation state, and sends typed user intents. The presentation is not an OS executor.
- **P4 contracts** (`docs/p4/`): draft APIs, Protobuf, IPC, database migrations, event and workflow contracts, context and tool schemas, security profiles, adapter rules, and compatibility/conformance records.
- **P5 plan and results** (`docs/p5/`): milestone plans, acceptance-condition mapping, provider evaluation, and the synthetic acceptance report.
- **Validation and toolchain** (`scripts/`, `toolchain/`, `.github/workflows/`): pinned contract-check tooling, migration/fixture validators, and CI.
- **Test vectors and evidence** (`testdata/`, `evidence/`): synthetic and public security vectors plus retained, redacted milestone results.

## Current status

P5 milestones M0–M5 are reported complete in the [acceptance report](docs/p5/acceptance-report.md). That report records 108 passing workspace tests and successful formatting, clippy, and contract-validation checks. The live environment conformance gates remain **Not Run**: they require execution on the target Linux/Hyprland workstation. P4 contracts remain Draft, and the repository does not claim general Linux, Windows, macOS, mobile, or production-provider support.

The current proof target is one Linux host, one display, and one Hyprland profile. The implemented adapter capability is intentionally narrow: same-display movement of a supported window. It does not imply arbitrary application control or generic input automation.

## Architecture at a glance

```text
Godot presentation
       │ typed events and state snapshots
       │ private, framed Unix socket
Rust device agent ── deterministic policy and local SQLite state
       │
       ├── conversation coordinator ── synthetic model/speech providers
       └── Hyprland adapter ── discover, authorize, dispatch, read back
```

The Rust agent owns local execution and checks authorization at the point of use. Model output is an untrusted proposal; it cannot grant itself permission. The adapter reports success only after an independent read-back confirms the result. A timeout or uncertain postcondition remains an unknown outcome that needs reconciliation. Window titles are redacted, and the P5 synthetic provider path is local-only.

These boundaries are described in more detail in the [blueprint](LIA-Project-Blueprint.md), the [P4 contract overview](docs/p4/README.md), and [P5 plan](docs/p5/README.md).

## Build and validate

The repository pins Rust via `rust-toolchain.toml` and Node/npm tooling in `toolchain/versions.toml` and `package-lock.json`. Use the documented bootstrap on a clean Linux checkout:

```sh
bash toolchain/bootstrap.sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The contract checks use the pinned npm dependencies:

```sh
npm ci
npm run check:openapi
npm run check:types
npm run check:json
npm run check:sqlite
npm run check:postgres
npm run check:proto
```

`check:proto` bootstraps the pinned `protoc` binary when needed. The checks cover OpenAPI bundling, TypeScript interfaces, JSON fixtures, SQLite and PostgreSQL migration chains, and Protobuf compilation. The PostgreSQL chain is checked by the validation harness; PostgreSQL is not required by the local P5 desktop runtime. See [toolchain notes](toolchain/README.md) for what each pin is for.

For the repository integrity check:

```sh
python3 scripts/check-blueprint-integrity.py
```

These commands validate code and artifacts. They do not, by themselves, pass the live conformance gates.

## Repository map

| Path | Contents |
| --- | --- |
| `crates/lia-device-agent/` | Rust agent implementation and unit/conformance tests |
| `apps/lia-presentation-godot/` | Godot project, IPC client, state machine, UI scripts, and client tests |
| `docs/p4/` | Draft integration contracts and schemas |
| `docs/p5/` | First-slice milestones, decisions, and acceptance report |
| `scripts/` | Fixture, migration, and blueprint integrity checkers |
| `toolchain/` | Bootstrap scripts, exact version record, and dependency policy |
| `testdata/` | Public standards-derived vectors and synthetic fixtures |
| `evidence/` | Redacted milestone and gate evidence, indexed by `evidence/README.md` |

## Design and implementation references

Start with these documents for the main project context:

- [Project blueprint](LIA-Project-Blueprint.md) — product baseline, requirements, architecture, invariants, scope, and decisions.
- [P5 plan](docs/p5/README.md) — approved Linux-first slice, milestone status, acceptance mapping, gate procedures, and open owner decisions.
- [P5 acceptance report](docs/p5/acceptance-report.md) — synthetic regression results, AC-001–AC-011 coverage, delivery log, and live-gate status.
- [P4 contract overview](docs/p4/README.md) — contract inventory, authority boundaries, security assumptions, and deferred integration work.
- [P4 conformance matrix](docs/p4/conformance/p4-conformance-matrix.md) and [compatibility matrix](docs/p4/compatibility-matrix.md) — proof procedures, current gate status, and stack status.
- [Toolchain guide](toolchain/README.md) and [version pins](toolchain/versions.toml) — bootstrap behavior and rationale for validation/runtime versions.
- [Evidence index](evidence/README.md) — where retained reports and test artifacts live.

The P4 package has focused documents for the [device/control-plane API](docs/p4/proto/lia/v1/control_plane.proto), [HTTP API](docs/p4/openapi/lia-v1.yaml), [presentation IPC](docs/p4/ipc/presentation-agent-v1.md), [security profile](docs/p4/security/security-profile-v1.md), [adapter conformance](docs/p4/adapters/adapter-conformance-v1.md), [context package](docs/p4/context/context-package-v1.md), [tool validation](docs/p4/tools/tool-validation-pipeline-v1.md), [workflow contracts](docs/p4/workflows/temporal-contracts-v1.md), [event contracts](docs/p4/events/event-contract-v1.md), [runtime profile](docs/p4/operations/runtime-profile-v1.md), and [release/supply-chain requirements](docs/p4/operations/supply-chain-release-v1.md). Their presence documents intended interfaces and controls; it does not mean those distributed services or providers are implemented.

Milestone task notes live in [`docs/p5/tasks/`](docs/p5/tasks/), provider research in [provider evaluation](docs/p5/provider-evaluation.md), and standards/security fixtures are explained in [`testdata/README.md`](testdata/README.md). Evidence directories include their own README files describing the run, inputs, and checksums.

## Scope and open decisions

The current slice does not include the P6 coordinator services (Temporal, NATS, PostgreSQL-backed control plane), Windows or macOS adapters, mobile, Home Assistant, or production model/speech providers. See the P5 plan for the deferred gate list and open owner decisions. The project license has not been selected; no license is implied by this repository.

Before changing behavior or claiming support, read [AGENTS.md](AGENTS.md). It records authority boundaries, privacy constraints, milestone scope, migration and evidence rules, and the definition of done. In particular, do not treat a draft contract, green structural check, or synthetic result as a live conformance pass.
