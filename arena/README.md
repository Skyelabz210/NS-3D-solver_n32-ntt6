# QMNF Unified Workspace
## Quantum-Modular Numerical Framework — Complete System

**Author:** Anthony Diaz (Acid / HackFate / Skyelabz210)  
**Date:** March 18, 2026  
**Tests:** 358 passed / 0 failed / 1 ignored  
**Lines:** 18,278 Rust + 8,972 Python + 486 Lean 4 = 27,736 total

---

## Quick Start

```bash
# Build and test everything
cargo test --workspace

# Run the 3D NS solver (takes ~25s for Biot-Savart at N=8)
cargo test -p nsgrid3d -- --nocapture

# Run just the fast crates
cargo test -p qmnf_primitives -p qedde_v2 -p mmbf_grover_bridge -p ns_winding_v2
```

## Architecture

```
16 Rust crates in a single Cargo workspace:

Layer 1 — Modular Primitives
  qmnf_primitives     CRTBigInt, K-Elimination, NTT, CRAM Grover, NINE65

Layer 2 — CRT Substrate  
  hdv_crt              Hyperdimensional CRT vectors
  wassan_holographic   144-band NTT holographic storage
  wassan_engine        WASSAN band arithmetic
  plmg_validator       Rail/void phase-lock validation
  maa_dh               Apollonian Double Helix (AHOP post-quantum)

Layer 3 — Schema Engine
  qedde_v2             Cell, CompositeSchema, HydraEngine, 7 PostHoc schemas

Layer 4 — Search
  mmbf_grover_bridge   Ramanujan-Grover formula (T_opt from c_p(d))

Layer 5 — Security
  loki_engine          Threat detection
  cdhs_telemetry       Health diagnostics

Layer 6 — Cognitive
  asve_engine          Perceive, hypothesize, learn
  time_crystal         Temporal coherence structures
  svd_engine           SVD decomposition

Layer 7 — Applications
  ns_winding_v2        2D NS: exact energy certificate, lifted winding
  nsgrid3d             3D NS: Biot-Savart, fused solver, regularity tracker
  hackfate_runtime     Full lifecycle runtime
```

## Key Results

- **K-Elimination:** Exact RNS division, 10^8+ operations, 0 errors (Lean 4 verified)
- **Hydra Sieve:** 79.8% twin prime elimination, zero false negatives  
- **Energy Certificate:** Skew-symmetric advection = integer zero (2D proved, 3D verified)
- **Ramanujan-Grover:** T_opt = ⌊π/4 · p / √(2p−1−c_p(d))⌋, 1801× speedup at p=10^6
- **One-Shot Learning:** 10/10 exact, 200/200 noisy on synthetic CRT classifier
- **Separation Theorems:** T8 Weil bound holds all primes 3-199, T10 Hydra optimal 15/15
- **Float Audit:** Zero f64 in any critical computation path

## Documentation

- `MMBF_v2.md` — Mathematical document: proved theorems, honest negatives, open problems
- `crates/qedde_v2/UNIFIED_ARCHITECTURE.md` — Engineering layer stack specification
- `lean/RamanujanHydra_v2_1_2.lean` — Lean 4 formalization (T6 suite, 486 lines, 0 sorry)

## Python Verification Suite

```bash
python3 python/separation/separation_theorems.py  # T8/T9/T10
python3 python/yukawa/cram_yukawa_schema.py        # Yukawa 5-stage pipeline
python3 python/hydra/hydra_sieve_v3.py             # Hydra sieve
```
