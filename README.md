# NS 3D N32 NTT6

3D Navier-Stokes solver workspace at grid resolution N=32, using a 6-prime
NTT spectral basis. Snapshot of the **P5B-9A gate-pass** state, 2026-04-29.

## What this is

A Cargo workspace (`arena/`) implementing a Navier-Stokes solver that uses
exact-integer arithmetic and a number-theoretic-transform-based spectral
representation. The gate test that this snapshot passes:

- `per_prime_spectral_residues_crt_consistent` — confirms 6-prime spectral
  computation gives CRT-consistent results across all `N³ = 32,768` cells
  for the Taylor-Green amp=100 test.
- 58/58 tests passing as of the gate.

## Layout

```
.
├── arena/                              ← the cargo workspace
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── asve_engine/
│   │   ├── cdhs_telemetry/
│   │   ├── cram_explorer/              ← local crate (apollonian, holodrive, k_elim, ...)
│   │   ├── d024_probe/                 ← attractor / corridor probes
│   │   └── ...
│   ├── README.md
│   └── SESSION_*.md                    ← per-session notes (D024, PUSH7-10)
├── ns3d_n32_ntt6_P5B9A_prereqs.rs      ← prerequisites for the P5B-9A path
├── P5B_DAG_P5B9A_GATE.md               ← master DAG, current architectural framing
├── PROPOSAL_P5B9A_EXECUTION.md         ← proposal for next implementation step
└── README.md                           ← this file
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
