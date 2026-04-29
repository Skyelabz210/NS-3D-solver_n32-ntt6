# NS 3D N32 NTT6

This repo bundles **two related-but-distinct systems** with shared genealogy.
Snapshot taken at the **P5B-9A gate-pass** state, 2026-04-29.

## What this is — two systems, one repo

The NS solver in this repo did not start as an NS solver. It grew out of
the **mother prime sieve** being pointed at fluid behavior in
Navier-Stokes — when new readings emerged, a solver was built around them.
The two systems share a structural skeleton because of that lineage; they
are not duplicates.

### System 1 — Mother Prime Sieve

The original sieve framework. Lives under `arena/qmnf_unified/` (and the
top-level `arena/prime_hunt/` mirror). This is the discovery tool: point
it at a problem, let structure emerge.

Includes:
- The full sieve crate set (qmnf_primitives, prime_hunt, hydra_v3,
  landau_siegel_chimera, mmbf_grover_bridge, quantum_continuum_torus,
  ramanujan_memory_lane, time_crystal, …)
- `docs/atlas/`, `docs/cram_definitive/`, `docs/wave3/` — the
  framework-underneath documentation (CRAM operator skill v2,
  Algebraic Measurement Framework, theorem stacks)
- `lean/` — Lean formalization seeds
- `python/` — research Python (resnet ablations, yukawa schema,
  hydra investigations, separation theorems)
- `skills/cram-operator-skill/`, `skills/prime-family-spacetime/` —
  Claude Code skill bundles
- `tools/python/` — sieve and analysis utilities

### System 2 — NS 3D N32 NTT6 Solver

The Navier-Stokes solver that developed after the sieve was pointed at NS
fluid. Lives under `arena/crates/`. Uses exact-integer arithmetic and a
6-prime NTT spectral representation.

Includes:
- `asve_engine`, `cdhs_telemetry` — runtime + telemetry
- `cram_explorer` (local crate, distinct from the cram-explorer skill in
  `~/.claude/skills/`) — apollonian, holodrive, k_elim, projector,
  shadow_ladder, torus
- `d024_probe` — attractor and corridor probes (D-024 work)
- `nsgrid3d` — the actual N=32 grid solver (biot_savart_ntt, fused_solver,
  regularity_tracker, …)
- `ns_v5` — NS regularity engine (claim registry, status matrix, kill
  ledger)
- `ns_winding_v2` — 2D NS exact-energy + lifted winding
- `qcid_ns_bridge` — Fp² dual-track NS bridge
- `qedde_v2`, `qedde_v3` — Q-Eddington diophantine
- `wassan_engine`, `wassan_holographic` — WASSAN HD
- `maa_dh` — AHOP / Apollonian Double Helix

The gate test that this snapshot passes is solver-side:

- `per_prime_spectral_residues_crt_consistent` — confirms 6-prime spectral
  computation gives CRT-consistent results across all `N³ = 32,768` cells
  for the Taylor-Green amp=100 test.
- 58/58 tests passing as of the gate.

### Why both live in this repo

Genealogical: the solver inherited the sieve's skeleton because the solver
was built *out of* the sieve's readings. Splitting them into separate
repos would erase the lineage. Apparent duplicates (e.g.,
`arena/crates/qmnf_primitives/` vs `arena/qmnf_unified/crates/qmnf_primitives/`)
are NOT duplicates — they are the sieve-side and solver-side instances of
shared substrate code, evolved along their own paths.

## Layout

```
.
├── README.md
├── .gitignore
├── arena/
│   ├── Cargo.toml
│   ├── README.md
│   ├── SESSION_*.md                    ← per-session notes (D024, PUSH7-10)
│   ├── executioner_dag.md
│   │
│   ├── crates/                         ← System 2: NS Solver
│   │   ├── asve_engine/
│   │   ├── cdhs_telemetry/
│   │   ├── cram_explorer/              ← local solver-side crate
│   │   ├── d024_probe/
│   │   ├── nsgrid3d/
│   │   ├── ns_v5/
│   │   ├── ns_winding_v2/
│   │   ├── qcid_ns_bridge/
│   │   ├── qedde_v2/, qedde_v3/
│   │   ├── wassan_engine/, wassan_holographic/
│   │   └── ... (~25 crates)
│   │
│   ├── qmnf_unified/                   ← System 1: Mother Prime Sieve
│   │   ├── crates/                     ← sieve-side instances of the substrate
│   │   ├── docs/atlas/                 ← CRAM operator skill, dangerous configs
│   │   ├── docs/cram_definitive/       ← Algebraic Measurement Framework, atlas
│   │   ├── docs/wave3/                 ← theorem stacks, blueprints
│   │   ├── lean/                       ← Lean formalization seeds
│   │   ├── python/                     ← resnet, yukawa, hydra, separation
│   │   ├── skills/                     ← Claude Code skill bundles
│   │   └── tools/python/               ← sieve + analysis utilities
│   │
│   └── prime_hunt/                     ← top-level mirror of sieve crate
│
├── ns3d_n32_ntt6_P5B9A_prereqs.rs      ← P5B-9A path prerequisites
├── P5B_DAG_P5B9A_GATE.md               ← master DAG (corrected architecture)
└── PROPOSAL_P5B9A_EXECUTION.md         ← next-step proposal
```

## Build / test

From the workspace directory:

```sh
cd arena
cargo build --workspace
cargo test --workspace
```

The gate test specifically:

```sh
cargo test -p d024_probe per_prime_spectral_residues_crt_consistent
```

## Architectural notes

See `P5B_DAG_P5B9A_GATE.md` for the current corrected architecture. Key points:

- **4p spectral witness basis:** `{769, 929, 1153, 1217}` — Garner4
  reconstruction of `raw_BS` inside `biot_savart_residue_native`.
- **6p division/state basis:** `{769, 929, 1153, 1217, 1409, 1601}` plus
  `{2³¹, 5⁹}` — K-Elim division quotients + ExtRes8 state representation.
- **Hard precondition** (documented and tested as a guard): if `raw_BS`
  exceeds `M_NTT_4 / 2 ≈ 5.01×10¹¹`, Garner4 aliases. The 6p basis does NOT
  remove the precondition — it inherits it.

## Floating-point posture

Exact integer arithmetic only. No `f32`, `f64`, no `as f64`, no `floor()`,
no `round()` in computation paths. Display-only conversions are tolerated.

## Status

P5B-9A gate: **PASS**. Next step in `PROPOSAL_P5B9A_EXECUTION.md`.

## License

Private. Not for redistribution.
