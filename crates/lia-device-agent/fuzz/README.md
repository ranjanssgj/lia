# Device-agent fuzz targets

The fuzz crate is isolated from the main Rust workspace so the desktop agent remains pinned to `rust-toolchain.toml` (Rust 1.99.1). The fuzz package has its own lockfile and dated nightly pin: `nightly-2026-11-17` with `llvm-tools-preview`, `cargo-fuzz` 0.13.2, and `libfuzzer-sys` 0.4.13.

The `ipc_frame` target feeds arbitrary bytes into the production framed IPC decoder. Four checked-in corpus seeds exercise valid hello, ready, user-event, and state-snapshot envelopes. The target asserts every accepted frame also passes envelope validation.

From `crates/lia-device-agent/`, install the pinned nightly and cargo-fuzz, then run:

```sh
rustup toolchain install nightly-2026-11-17 --profile minimal --component llvm-tools-preview
cargo install cargo-fuzz --version 0.13.2 --locked
cargo +nightly-2026-11-17 fuzz run ipc_frame -- -max_total_time=60 -max_len=262148 -timeout=5 -verbosity=0 -print_final_stats=1
```

The target uses libFuzzer coverage guidance and AddressSanitizer. Generated corpus entries and crash artifacts are synthetic. Review crashes before updating any conformance status; a bounded smoke run alone does not pass IPC-01.
