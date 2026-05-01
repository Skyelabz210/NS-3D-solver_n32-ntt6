# Workspace Manifest

Repository: `C:\Users\hackf\Projects\ns3d-n32-ntt6`

This manifest is the root orientation file for the current workspace. It is
meant to answer three questions quickly:

1. what lives here
2. which parts are active
3. where Wasan fits into the whole system

## Repository identity

This repository holds two related lines with shared genealogy:

- the prime-substrate / CRAM / sieve / chimera line
- the exact-integer Navier-Stokes line built from that substrate

That is why multiple trees that look similar at first glance live together.
They are related branches, mirrors, or adapted instances, not meaningless
duplication.

## Root inventory

- `README.md`
  - current top-level orientation
- `WORKSPACE_MANIFEST.md`
  - this file
- `AGENTS.md`
  - repo-native guidance for agents
- `P5B_DAG_P5B9A_GATE.md`
  - current gate-state architecture notes
- `PROPOSAL_P5B9A_EXECUTION.md`
  - next-step execution document
- `CONSOLIDATED_WORK_LOG.md`
  - cross-workspace consolidation notes
- `GITHUB_REPOSITORY_INVENTORY.md`
  - wider repo family inventory
- `QWEN.md`
  - broad generated repository summary
- `imports/`
  - incoming source material
- `arena/`
  - active Cargo workspace

## Active workspace

Path: `arena/`

The active Cargo workspace currently includes:

- solver and probes
  - `ns_v5`
  - `nsgrid3d`
  - `d024_probe`
  - `ns_winding_v2`
- substrate and arithmetic
  - `qmnf_primitives`
  - `qmnf_crtbigint`
  - `hdv_crt`
  - `k_elim_period`
- bridges and chimeras
  - `qcid_ns_bridge`
  - `landau_siegel_chimera`
  - `hydra_v3`
  - `prime_hunt`
  - `qedde_v2`
  - `qedde_v3`
- Wasan and persistence
  - `wassan_holographic`
  - `wassan_engine`
- runtime and validation
  - `asve_engine`
  - `cdhs_telemetry`
  - `loki_engine`
  - `hackfate_runtime`
  - `plmg_validator`
  - `time_crystal`
  - `svd_engine`
  - `maa_dh`
  - `mmbf_grover_bridge`
  - `quantum_continuum_torus`
  - `ramanujan_memory_lane`
  - `sanity_crt`

## Companion research tree

Path: `arena/qmnf_unified/`

This is the sieve-side and research-side companion tree. It contains:

- mirrored and companion crates
- docs and theorem stacks
- Lean seeds
- Python research tools
- skill bundles
- analysis tooling

This tree should be read as part of the same family, but not assumed to be
build-identical to `arena/crates/`.

## Wasan family map

Human-facing spelling: **Wasan**
Internal crate naming: `wassan_*`

### Holographic layer

Path: `arena/crates/wassan_holographic`

Primary mechanics:

- 144 data bands
- 256 total sectors
- 112 parity bands
- 56-band correction ceiling
- 432-byte hologram capacity
- exact modular NTT per CRT lane
- syndrome detection from inverse-NTT parity inspection
- sector corruption repair by re-encoding or projection

Key code surfaces:

- `src/lib.rs`
  - `HoloNttCtx`
  - `WassanHologram`
  - `compute_syndrome`
  - `self_correct`
  - `recover_from_sectors_only`
  - `AttractorBank`

### Engine layer

Path: `arena/crates/wassan_engine`

Primary mechanics:

- basin creation
- store / retrieve-by-id
- retrieve-by-content
- convergence tracking
- stability scoring

Key code surfaces:

- `src/lib.rs`
  - `WassanMemoryEngine`
  - `ConvergenceTracker`

### NS persistence bridge

Paths:

- `arena/crates/ns_v5/src/wassan.rs`
- `arena/crates/ns_v5/src/ns3d_wassan.rs`

Primary mechanics:

- persists recumbent structure, not flattened scalar values
- preserves tier placement
- carries overflow history forward
- supports WASSAN-backed NS stepping and snapshot diagnostics

## Current Wasan benchmark route

Command:

```powershell
cd C:\Users\hackf\Projects\ns3d-n32-ntt6\arena
cargo run -p wassan_engine --bin wasan_bench --release
```

Measured operations:

- encode
- decode
- verified decode
- syndrome inspection
- self-correction
- basin store
- basin retrieval

Optional args:

- `--samples <n>`
- `--bank-size <n>`

## Core working commands

```powershell
cd C:\Users\hackf\Projects\ns3d-n32-ntt6\arena
cargo build --workspace
cargo test --workspace
cargo test -p ns_regularity cram_div_u128 --lib
cargo test -p ns_regularity ns3d_n32_crt_v4 --lib
cargo test -p wassan_holographic --lib
cargo test -p wassan_engine --lib
cargo run -p d024_probe --bin v4_vs_i128_ref --release
cargo run -p wassan_engine --bin wasan_bench --release
```

## Working-tree reality

This repository is live and may contain:

- active solver edits
- untracked imported files
- generated documentation
- branch-local probes

Documentation or benchmark additions should be additive and should not assume
the working tree is clean.
