# Push 9: Canonical Solver Reality Check

## What this is

Replacement for Push 7/8 using the workspace's actual canonical NS
solver. Per operator correction: **use OUR system — K-Elimination
fused piggy-back division and CRAM operator transforms, not Garner**
(K-Elim does what Garner does AND more — it returns the winding number
as a first-class object, and the iterated K-Elim reconstruction
already delivers signed centered i64 by construction).

The workspace `ns_v5::ns3d::ns_step` IS this. It uses:
- 4 NTT primes `{73, 89, 97, 113}`, NTT_MODULUS = 71,213,617
- `garner4` (= iterated K-Elim) reconstruction, returning centered i64
- i128 intermediates, **truncating integer division** `/` after K-Elim
  reconstruction — this is the "fused piggy-back": K-Elim returns a
  signed integer in i64, ordinary `/` is then truncating division
- CFL-bounded params via `NsParams::cfl_max(nu_scaled)`

## Push 7/8 retraction confirmed

Push 7/8 used a different `CRTBigInt` (8 huge ~2^63 primes with a
separate `sign: i8` field) and modular-inverse "division." The kill-
tests in `crates/d024_probe/src/bin/kill_tests.rs` showed:

- `1 + (-1) = Some(2)` (broken Add)
- `2 - 5 = None` (broken Sub)
- `(-3) + 3 = Some(-6)` (broken sign propagation)
- `5 * SCALE⁻¹ mod p` is a random-looking residue, NOT 0 (modular
  inverse is not truncating division)
- Garner uses `wrapping_add`/`wrapping_mul` — silent u128 overflow

So every step of Push 7/8 was running on broken arithmetic. The
"matched 0.31 log10/step" in Push 8 just confirmed both b-step and
canonical-step harnesses shared the same broken CRTBigInt.

## CFL framing in Push 8 was inverted

`NsParams::cfl_max(nu_scaled=10⁶)` returns `dt_latt = 1066` (computed
as `(NS_SCALE² · N²) / (6 · nu_scaled) = 6.4×10⁹ / 6×10⁶`).

Push 7/8 used `dt_latt = 100`. That is **10× UNDER** the CFL bound,
not over it. Push 8's "CFL violation" diagnosis was wrong direction.

## Push 9 results — canonical solver, same ICs

`NsParams::cfl_max(nu_scaled=10⁶)` → `dt_latt=1066`, CFL ok = true.
100 steps each.

| Test                  | Initial L²      | Final L²              | factor/step | saturated |
|-----------------------|-----------------|-----------------------|-------------|-----------|
| TG amp=10000          | 2.55×10¹⁰       | 3.02×10⁶              | 0.913       | no        |
| TG amp=1000           | 2.54×10⁸        | 2.18×10⁶              | 0.954       | no        |
| TG amp=100            | 2.51×10⁶        | 2.32×10⁶              | 0.999       | no        |
| ABC amp=10000         | 1.53×10¹¹       | 1.70×10³⁸ (i128 sat)  | 1.864       | **YES**   |
| ABC amp=100           | 1.52×10⁷        | 1.70×10³⁸ (i128 sat)  | 2.044       | **YES**   |
| Random amp=10000      | 4.08×10¹⁰       | 1.70×10³⁸ (i128 sat)  | 1.889       | **YES**   |
| Random amp=100        | 4.01×10⁶        | 1.70×10³⁸ (i128 sat)  | 2.071       | **YES**   |

Two distinct dynamical regimes show up cleanly:

### Taylor-Green: bounded, contracting

L² goes from 2.55×10¹⁰ down to ~3×10⁶ within ~30 steps and stays there.
L∞ goes from 10⁴ to 87 and stops moving. NO saturation. The integrator
finds an attracting state (the symmetric viscous-equilibrium structure
TG was designed around) and stays there.

This **directly contradicts** Push 7/8's claim of uniform 0.31 log10/step
growth across all backgrounds including TG. The canonical solver
contracts on TG.

### ABC and Random: blowup to clamp by step ~10

L² hits 1.70×10³⁸ ≈ i128::MAX (the `l2_sq` saturation sentinel) within
about 10 steps. L∞ hits 9.22×10¹⁸ ≈ i64::MAX/2. The per-step factors
~1.9–2.1 are NOT physical NS dynamics — they're the signature of
saturating clamps comparing pegged values.

The canonical `ns_step` has a saturating clamp at line 730:
`out[i][j][k] = next.clamp(i64::MIN as i128, i64::MAX as i128) as i64`.
This silently absorbs unphysical blowup on asymmetric initial conditions
at CFL_max. Workspace tests pass because they exercise TG-class cases.

## What this means

For Push 7/8 conclusions:
- "Extension introduces new instability — REFUTED" → **withdrawn.** The
  refutation was based on broken arithmetic.
- "Growth is integrator instability — CONFIRMED" → **withdrawn.** The
  matched growth rates compared two broken harnesses.
- "Substrate validated by 2^504 capacity unused" → **withdrawn.** The
  substrate's signed arithmetic was the actual problem; magnitude
  capacity was never the issue.

For D-024:
- K_b = 0: still proved (Push 1, valid i64 arithmetic).
- b = 0 invariant: still proved (algebraic).
- b = 0 attractor (linearized): Push 4 needs re-audit; uses same
  truncating-renormalize concern.
- Extension instability: untested. The hypothesis is open.
- Strong form (b=0 globally attracting): untested.

For the workspace itself — a separate operational finding:
- The canonical N=8 `ns_step` saturates to i64::MAX on ABC and Random
  initial conditions at CFL_max within ~10 steps. The clamp at line 730
  conceals it. Tests pass because they test TG.
- For ABC/Random, the solver corridor needs either smaller dt (sub-CFL)
  or a wider-arithmetic path (i128 throughout, or the F_p² dual-track
  in `qcid_ns_bridge`).

## What the K-Elim substrate buys us (and what it doesn't)

The user's correction was operationally correct. K-Elim/Garner4
delivers:
- **Signed integer reconstruction** automatically (centered i64 from
  the M/2 split — value > M/2 is treated as negative)
- **Truncating integer division** trivially after reconstruction
  (because the value is now a plain signed integer)
- **The winding number** as a first-class object — useful for tracking
  exactly how many M-cycles a value has accumulated through

What it does NOT solve:
- **The corridor**: NTT_MODULUS = 7×10⁷ for N=8 means the solver still
  has to keep all reconstructed values in `[-M/2, M/2]`. ABC and Random
  blow past this within ~10 steps at canonical params. A wider corridor
  needs either bigger NTT_MODULUS (more or larger NTT primes) or
  different time integration.
- **The clamp**: `ns_step` itself uses i64 fields and clamps. Nothing
  in the K-Elim substrate forces the solver to use wider local types
  for the per-cell update; that's a solver design choice independent
  of the reconstruction layer.

## Files

```
crates/d024_probe/src/bin/canonical_p9.rs   # Push 9 canonical solver runner
crates/d024_probe/src/bin/kill_tests.rs     # Phase 1 kill-tests
```

## What's next

If D-024 is to be tested with confidence:

1. **Wider-corridor solver path.** The `qcid_ns_bridge` crate has F_p²
   dual-track (a, b) plus winding tracking. That looks like the right
   substrate for both the standard NS and the b-extension to coexist
   without saturation.
2. **Sub-CFL integration for ABC/Random.** Run `ns_step_substep` (which
   exists in the workspace) with k_sub > 1 to step at dt = dt_latt / k_sub
   and check whether the blowup is genuine or numerical.
3. **Replace clamp with corridor-exit detection.** Make the saturation a
   hard fail-stop, not a silent absorb. Then we know exactly when
   integration left the corridor and can report it as a negative
   (which is what the workspace l2_sq sentinel does for the L²
   measurement, but `ns_step` itself absorbs silently).

These are the right next experiments. Not more attractor claims on
broken arithmetic.
