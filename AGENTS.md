# Repository Guidelines

## Project Structure & Module Organization

This repository contains two related systems. The active Cargo workspace is `arena/`. Solver-side Navier-Stokes work lives in `arena/crates/`, especially `ns_v5`, `nsgrid3d`, `d024_probe`, and bridge/support crates. Sieve-side and research material lives in `arena/qmnf_unified/`, including `crates/`, `docs/`, `lean/`, `python/`, `skills/`, and `tools/python/`. Root-level files such as `README.md`, `P5B_DAG_P5B9A_GATE.md`, and `PROPOSAL_P5B9A_EXECUTION.md` document current architecture and session direction. `imports/` holds incoming snapshot material; integrate deliberately rather than treating it as build output.

## Build, Test, and Development Commands

Run Cargo commands from `arena/`.

```sh
cargo build --workspace
cargo test --workspace
cargo test -p ns_regularity cram_div_u128 --lib
cargo test -p ns_regularity ns3d_n32_crt_v4 --lib
cargo run -p d024_probe --bin v4_vs_i128_ref --release
```

`cargo build --workspace` checks all workspace crates. `cargo test --workspace` runs the full suite. The targeted `ns_regularity` tests verify the u128 substrate and v4 NS step. `v4_vs_i128_ref` is the current imported verification binary for the u128 v4 path.

## Coding Style & Naming Conventions

Rust uses edition 2021 and standard `rustfmt` defaults; keep four-space indentation and idiomatic snake_case for modules, functions, and variables. Preserve existing domain names when they are already established, such as `ns3d_n32_crt_v4`, `CRTVortExtU128`, and `ExtResU128`. Keep computation paths exact-integer only: no `f32`, `f64`, `as f64`, `floor()`, or `round()` except display-only reporting.

## Testing Guidelines

Most tests are inline `#[cfg(test)] mod tests` blocks. Add focused tests beside the code they validate, and use descriptive names like `div_by_s_squared_at_step11_magnitude`. For solver changes, run the relevant crate tests plus any gate or comparison binary that exercises the changed corridor.

## Commit & Pull Request Guidelines

Recent commits use concise imperative summaries, for example `Reframe README: two distinct systems, not one solver`. Keep commits scoped to one architectural or behavioral change. PRs should explain the changed subsystem, list commands run, note any remaining warnings, and link the relevant session or proposal document when applicable.

## Agent-Specific Instructions

Do not collapse solver-side and sieve-side mirrors as duplicates. When importing from `imports/`, compare API expectations first, update module/bin wiring, and preserve unrelated local work.
