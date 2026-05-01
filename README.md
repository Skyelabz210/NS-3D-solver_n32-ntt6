# ns3d-n32-ntt6

Private research repository for two related systems with shared genealogy:

1. the mother-prime and CRAM substrate line
2. the exact-integer Navier-Stokes line built from that substrate

This is not a single-purpose solver repo. It is a living workspace where the
substrate, the stress-test application, and the bridging storage/runtime layers
coexist because they were built out of the same line of work.

GitHub remote: `https://github.com/Skyelabz210/ns3d-n32-ntt6`

## Orientation

- `arena/` is the active Cargo workspace.
- `arena/crates/` is the active solver-side and runtime-side crate set.
- `arena/qmnf_unified/` is the sieve-side and research-side companion tree.
- `imports/` is incoming source material and should be treated as deliberate
  integration input, not build output.
- Root docs such as `P5B_DAG_P5B9A_GATE.md`,
  `PROPOSAL_P5B9A_EXECUTION.md`, and `CONSOLIDATED_WORK_LOG.md` record the
  current state of the line.

For a fuller map, see `WORKSPACE_MANIFEST.md`.

## What lives here

### 1. Prime-substrate and CRAM line

This side of the repo includes the mother-prime sieve, QMNF/CRAM substrate,
operator families, chimera systems, Grover-on-classical-torus work, storage
and runtime layers, and related theorem stacks and formalization seeds.

Representative crates and trees:

- `arena/crates/qmnf_primitives`
- `arena/crates/prime_hunt`
- `arena/crates/hydra_v3`
- `arena/crates/landau_siegel_chimera`
- `arena/crates/qcid_ns_bridge`
- `arena/crates/qedde_v2`
- `arena/crates/qedde_v3`
- `arena/qmnf_unified/docs`
- `arena/qmnf_unified/lean`
- `arena/qmnf_unified/python`

### 2. NS exact-integer line

This side of the repo contains the active Navier-Stokes workspaces and support
crates built on exact modular arithmetic, CRT, K-Elimination, and NTT-based
representations.

Representative crates:

- `arena/crates/ns_v5`
- `arena/crates/nsgrid3d`
- `arena/crates/d024_probe`
- `arena/crates/ns_winding_v2`
- `arena/crates/plmg_validator`
- `arena/crates/asve_engine`
- `arena/crates/cdhs_telemetry`

Current active import lineage matters here: the `u128`/`v4` path in
`imports/` and `arena/crates/ns_v5/src/ns3d_n32_crt_v4.rs` is part of the
live engineering line.

## Wasan HD

Human-facing spelling in this repo is **Wasan**, as in sacred mathematics.
Current crate names still use the established internal identifiers
`wassan_holographic` and `wassan_engine`.

The Wasan line has two connected layers:

- `arena/crates/wassan_holographic`
  - exact NTT hologram construction, sector-domain storage, syndrome checks,
    self-correction, and phase-locked retrieval scoring
- `arena/crates/wassan_engine`
  - basin-oriented storage and retrieval wrapper built on top of the
    holographic layer

There is also a distinct NS persistence bridge in:

- `arena/crates/ns_v5/src/wassan.rs`
- `arena/crates/ns_v5/src/ns3d_wassan.rs`

That layer uses Wasan to persist full recumbent structure for NS cells rather
than flattening everything into ephemeral integers.

## Wasan mechanics

The current holographic mechanics are concrete and inspectable in code:

- `NUM_DATA_BANDS = 144`
- `NTT_SIZE = 256`
- `NUM_PARITY_BANDS = 112`
- `MAX_CORRECTABLE_ERRORS = 56`
- `BYTES_PER_COEFF = 3`
- `HOLOGRAM_CAPACITY = 432` bytes per hologram

Pipeline:

1. ingest bytes into 144 coefficient bands
2. zero-pad into a 256-slot polynomial
3. run exact modular forward NTT lane-by-lane
4. store 256 sector evaluations
5. retrieve by sector-domain coherence or exact label lookup
6. detect corruption by inverse-NTT parity inspection
7. restore sectors from authoritative bands or project from sectors back into
   the valid codeword subspace

There are two different persistence ideas in play:

- **holographic persistence**
  - each sector depends on all encoded bands
  - parity bands give a correction surface
  - attractor-bank retrieval matches by coherence, not by linear scan of
    decoded strings
- **structural persistence**
  - NS-side Wasan persists recumbent tier geometry, overflow history, and
    exact lane structure
  - `ns_step_wassan()` keeps the field inside a write/read trajectory instead
    of discarding structure at each step

## Novel functionality currently present

The Wasan implementation here is not just "storage with a math theme". The
novel functional surface currently visible in the repo is:

- exact modular holographic encoding using multiple NTT-friendly CRT lanes
- sector-domain storage where every sector depends on the whole encoded object
- syndrome-based corruption detection without floating point
- self-correction by re-encoding from authoritative bands or projecting
  sectors back into valid parity space
- phase-locked retrieval through coherence scoring
- basin-based memory organization in `wassan_engine`
- NS structural persistence via `WassanDrive` and `WassanVortField`
- exact preservation of recumbent tier placement, overflow history, and
  full-lane state across NS write/read cycles

## Build and test

Run Cargo commands from `arena/`.

```powershell
cd C:\Users\hackf\Projects\ns3d-n32-ntt6\arena
cargo build --workspace
cargo test --workspace
```

Useful focused commands:

```powershell
cargo test -p ns_regularity cram_div_u128 --lib
cargo test -p ns_regularity ns3d_n32_crt_v4 --lib
cargo run -p d024_probe --bin v4_vs_i128_ref --release
cargo test -p wassan_holographic --lib
cargo test -p wassan_engine --lib
```

## Wasan benchmark path

There was no dedicated benchmark harness for the Wasan crates before this pass.
The repo now has a simple release-mode benchmark binary:

```powershell
cd C:\Users\hackf\Projects\ns3d-n32-ntt6\arena
cargo run -p wassan_engine --bin wasan_bench --release
```

Optional knobs:

```powershell
cargo run -p wassan_engine --bin wasan_bench --release -- --samples 120 --bank-size 96
```

The benchmark reports timings for:

- hologram encode
- decode
- verified decode
- syndrome checks
- self-correction after deterministic sector corruption
- basin store throughput
- basin retrieval

This gives you a repeatable local measurement path without adding an external
benchmarking framework.

## Ground rules

- exact integer arithmetic only on computation paths
- do not treat sieve-side and solver-side mirrors as trivial duplicates
- do not treat `imports/` as disposable noise
- use the workspace lineage and theorem lineage when consolidating docs

## Status

This repo is active and intentionally dirty. New documentation and benchmark
work should be additive and should not rewrite unrelated solver changes in the
working tree.
