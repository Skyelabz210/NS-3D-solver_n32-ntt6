# GitHub Repository Inventory

Read-only inventory collected with `gh repo list --limit 100 --json nameWithOwner,description,isPrivate,updatedAt,url` on 2026-04-30.

## Active Spine

| Repository | Visibility | Updated | Role |
|---|---:|---:|---|
| `Skyelabz210/ns3d-n32-ntt6` | private | 2026-04-29 | Current exact-integer NS N32/NTT6 snapshot; P5B-9A gate-pass line. |
| `Skyelabz210/NS-Regularity` | private | 2026-04-22 | Earlier NS regularity framework via exact-integer CRT arithmetic. |
| `Skyelabz210/ns-lean` | private | 2026-04-24 | Lean-side NS/proof work. |
| `Skyelabz210/NINE65_v7` | private | 2026-03-16 | Bootstrap-complete FHE with K-Elimination; pre-CRAM upgrade target. |
| `Skyelabz210/QMNF_System` | private | 2026-03-12 | Integer-only QMNF architecture; likely bridge between NINE65, CRAM, and applications. |
| `Skyelabz210/NINE65-Rust-Guide` | public | 2026-04-24 | Public guide/reference for NINE65. |

## FHE / Cryptographic Lineage

| Repository | Visibility | Updated | Role |
|---|---:|---:|---|
| `Skyelabz210/NINE65_v6_a_Clockwork_Prime` | private | 2026-02-24 | Hardened v6 lineage: GRO timing gates and entropy monitoring. |
| `Skyelabz210/NINE65-v6-a-Clockwork-Prime` | private | 2026-02-16 | Alternate v6 lineage: bootstrap-free FHE with circular security and K-Elim. |
| `Skyelabz210/NINE65-v5` | private | 2026-02-11 | v5 bootstrap-free FHE with Coq/Lean4 proofs. |
| `Skyelabz210/NINE65-v5-proofstack` | private | 2026-02-08 | v5 proofstack archive. |
| `Skyelabz210/MANA-private` | private | 2026-02-11 | MANA FHE private development and cryptanalysis. |
| `Skyelabz210/MANA-definitive` | private | 2026-01-19 | MANA definitive build with 14 verified innovations. |
| `Skyelabz210/MANA-pages` | private | 2026-01-05 | MANA showcase. |
| `Skyelabz210/BlindRoute` | private | 2026-02-27 | FHE-encrypted messaging on NINE65 BFV. |
| `Skyelabz210/Loki5-Cryptographic-Framework` | private | 2026-02-11 | Quantum-resilient crypto stack; GRO concepts feed NINE65. |

## Formal / Primitive Repositories

| Repository | Visibility | Updated | Role |
|---|---:|---:|---|
| `Skyelabz210/qmnf-lean-coq-security-proofs` | public | 2026-02-13 | Public Lean/Coq security proof repository. |
| `Skyelabz210/k-elimination-lean4` | private | 2026-02-06 | Formal verification of K-Elimination for RNS division. |
| `Skyelabz210/exact-transcendentals` | private | 2026-02-11 | Integer-only CORDIC/AGM/binary-splitting transcendentals. |
| `Skyelabz210/Quantum_Modular_Numeric_Framework` | private | 2025-11-06 | Earlier/alternate QMNF repository. |

## Application / Substrate Repositories

| Repository | Visibility | Updated | Role |
|---|---:|---:|---|
| `Skyelabz210/MYSTIC` | private | 2026-02-05 | MYSTIC foundation substrate. |
| `Skyelabz210/ENHANCE` | private | 2026-02-11 | Integer-only image enhancement via QMNF. |
| `Skyelabz210/Spanky` | private | 2026-01-08 | Environmental futurecast system. |
| `Skyelabz210/magen-collab` | private | 2026-02-06 | Collaboration repository; needs audit. |

## Consolidation Notes

NINE65 is not just an isolated FHE repo. The GitHub layout shows a lineage: NINE65 v5/v6/v7, MANA, K-Elim formal proofs, QMNF, exact transcendentals, and application repos. The CRAM upgrade should target `NINE65_v7` first, with `QMNF_System`, `MANA-private`, `MANA-definitive`, `k-elimination-lean4`, and `exact-transcendentals` treated as supporting workspaces.

For mobile FHE, the first audit target is `NINE65_v7` arithmetic and operation layers:

- `crates/nine65/src/arithmetic/` - Barrett, Montgomery, NTT, RNS, K-Elimination, exact divider.
- `crates/nine65/src/ops/rns_fhe.rs` and `ops/rns_mul.rs` - DualRNS FHE multiplication and rescale surface.
- `crates/nine65/benches/timing.rs` - existing benchmark hooks for K-Elim, NTT, and RNS rescale.
- `crates/mana/` - lane-parallel accelerator path.
- `crates/nine65-wasm`, `crates/nine65-ffi`, and service bindings - deployment surfaces.
