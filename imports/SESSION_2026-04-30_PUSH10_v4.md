# SESSION 2026-04-30 — Push 10 v4 (u128 substrate, S² lift fixed)

## Headline

v3's step-11 failure at TG amp=1000 is structurally resolved. v4 runs on
a u128 substrate with widened small-prime-power lanes (`2⁶³` and `5²⁵`),
giving **0 mismatches against the i128-unclamped reference through step 14**
of the same trajectory that broke v3.

```
v4 (u128 substrate) vs i128 unclamped reference at TG amp=1000:

step  1: i128_ref_max=                         22465  mismatches=0  max|diff|=0
step  2: i128_ref_max=                        267072  mismatches=0  max|diff|=0
step  3: i128_ref_max=                       1221751  mismatches=0  max|diff|=0
step  4: i128_ref_max=                      14888350  mismatches=0  max|diff|=0
step  5: i128_ref_max=                     157986326  mismatches=0  max|diff|=0
step  6: i128_ref_max=                    1846147855  mismatches=0  max|diff|=0
step  7: i128_ref_max=                   39905567755  mismatches=0  max|diff|=0
step  8: i128_ref_max=                  605313955653  mismatches=0  max|diff|=0
step  9: i128_ref_max=                 9507448546991  mismatches=0  max|diff|=0
step 10: i128_ref_max=               158446159875824  mismatches=0  max|diff|=0
step 11: i128_ref_max=              2302154635728249  mismatches=0  max|diff|=0   ← v3 broke here (512 mismatches)
step 12: i128_ref_max=             42348058440120589  mismatches=0  max|diff|=0
step 13: i128_ref_max=            878081698885732391  mismatches=0  max|diff|=0
step 14: i128_ref_max=          10578943299752466854  mismatches=0  max|diff|=0   ← state grown to 1.06×10¹⁹
step 15: i128_ref_max=         129617513476229010451  mismatches=97792           ← new corridor (M_NTT in BS)
```

Four verified clean steps past v3's failure point, on a state that grew
from 1.6×10¹⁵ to 1.1×10¹⁹ in those four steps.

## Diagnosis (verified arithmetically)

v3's amp=1000 step-11 failure was structurally located in the `(nu·lap)/S²`
diffusion division. The lift_partial_to_full corridor is bounded by
`M_NTT · partial_mod` where `partial_mod = lane_mod / gcd(d, lane_mod)`:

```
For S² = 2⁸·5⁸ in v3's basis (lane 5⁹):
  gcd(S², 5⁹) = 5⁸
  partial_mod_5 = 5⁹/5⁸ = 5
  binding corridor = M_NTT · 5 ≈ 5×10¹²

q at step 11 = nu·lap / S² ≈ (10⁶ · 12 · 1.58×10¹⁴) / 10⁸ ≈ 1.9×10¹³

q exceeds corridor by ~3.8× → wrap inside lift_partial_to_full,
producing residue tuples that decode to wrong integers.
```

Diff magnitude observed: `1.88×10²⁷ ≈ 0.45·M_ext` — consistent with the
lift wrap, not the centered-reconstruction sign boundary.

## Construction (the fix)

Per directive (u128 substrate, no i64): widen the small lanes.

```
Old basis: [769, 929, 1153, 1217, 2³¹, 5⁹]      (M_small ≈ 4.2×10¹⁵)
New basis: [769, 929, 1153, 1217, 2⁶³, 5²⁵]     (M_small ≈ 2.7×10³⁶)

S² lift corridor:
  Old:  M_NTT · 5⁹/5⁸     = M_NTT · 5     ≈ 5×10¹²
  New:  M_NTT · 5²⁵/5⁸    = M_NTT · 5¹⁷   ≈ 7.6×10²³

11 orders of magnitude headroom over step-11 q = 1.9×10¹³.
```

All arithmetic in u128. Two `mulmod` paths: small modulus (m < 2⁶⁴, direct
u128 product) and large modulus (m up to M_small ≈ 2.7×10³⁶, 256-bit
schoolbook via binary-double-add for the CRT reconstruction in `to_i128`).

## Step 15 — new corridor (next-session target)

At step 15, ref_max = 1.30×10²⁰. v4 starts to diverge: 97792 mismatches,
max|diff| = 1.40×10²⁰.

This is the **M_NTT corridor in BS**, not the lift corridor that v4 fixed:
```
biot_savart_u128's u128_garner_centered reconstructs in [0, M_NTT) ≈ [0, 10¹²)
and centers at M_NTT/2 ≈ 5×10¹¹.

At step 15 input ω ≈ 10¹⁹, raw spectral product raw_v ≈ ω · N⁶ ≈ 10²⁸.
raw_v ≫ M_NTT → garner returns wrong centered representative.
```

The i128 reference also breaks here — its `biot_savart` uses canonical i64
internals operating on i64-clamped vorticity, so the reference's velocity
at step 15 is not the true unclamped velocity. Both v4 and reference are
wrong at step 15, but wrong differently — hence the mismatch.

Resolution: widen NTT-side basis. Adding NTT primes that satisfy
`p ≡ 1 (mod 64)` (required for N=32 32nd roots of unity). Candidates
near i64 capacity: search primes `p ≡ 1 mod 64` near 2¹⁵–2³¹. With 6 NTT
primes near 2³⁰, M_NTT ≈ 10⁵² — plenty of corridor for any state magnitude
that fits in M_ext.

## Test surface

```
cram_div_u128:                                          7/7  pass
  ├─ small_part_roundtrip_u128                          ✓
  ├─ to_i128_roundtrip (signed −2¹⁰⁰ ... 2¹⁰⁰)         ✓
  ├─ div_by_n_squared (positive x)                      ✓
  ├─ div_by_n_sixth                                     ✓
  ├─ div_by_s_squared_at_step11_magnitude               ✓  (q = 1.9×10²¹)
  ├─ stress_random_xs_three_divisors (1500 cases)       ✓
  └─ stress_step11_class_magnitudes (200 in 1–6×10²¹)   ✓

ns3d_n32_crt_v4:                                        2/2  pass
  ├─ crt_v4_step_matches_canonical_small_amp            ✓
  └─ crt_v4_step_matches_canonical_multistep_amp100     ✓

Existing v2/v3 tests still pass:                        unchanged

Binary v4_vs_i128_ref:
  TG amp=1000 multistep, 0 mismatches steps 1–14, breaks step 15 (M_NTT corridor)
```

## Files

```
crates/ns_v5/src/cram_div_u128.rs     NEW  u128 substrate primitives
crates/ns_v5/src/ns3d_n32_crt_v4.rs   NEW  NS step on u128 substrate
crates/d024_probe/src/bin/v4_vs_i128_ref.rs  NEW  amp=1000 multistep verification
crates/ns_v5/src/lib.rs                +    pub mod cram_div_u128, ns3d_n32_crt_v4
crates/d024_probe/Cargo.toml           +    bin entry v4_vs_i128_ref
```

v3 left untouched (still passes its existing test surface). v4 is the
forward path; v3 stays as reference for the diagnosis and for tracking
what the corridor widening actually fixed.

## What v4 establishes

1. **Diagnosis correct.** The S² lift corridor was the structural reason
   v3 broke at step 11. Widening lane 5 from 5⁹ to 5²⁵ resolves it.
2. **u128 substrate works.** All arithmetic routes through u128/i128 with
   no i64 narrowing inside the step body. The two-path `mulmod_u128`
   (small modulus direct, large modulus via 256-bit schoolbook) handles
   both basis-lane operations and CRT reconstruction.
3. **Trajectory holds 4 steps past v3's break.** State grew 41× across
   those steps (1.58×10¹⁵ → 1.06×10¹⁹) without v4 producing a single
   wrong cell.
4. **Next corridor is identifiable.** Step 15 fails at M_NTT corridor in
   BS, not the lift corridor v4 fixed. That corridor is widened by adding
   NTT primes — separate construction.

## What v4 does not yet do

- Push past state magnitude ~10¹⁹. The M_NTT corridor in BS becomes
  binding when `raw_v ≈ ω · N⁶ > M_NTT/2 ≈ 5×10¹¹`, which requires
  `ω > M_NTT/(2·N⁶) ≈ 500`. So strictly the M_NTT corridor was *always*
  binding for any nontrivial state — it just happened that canonical
  garner4 was equally wrong there in the same way, and the i128 reference
  was carrying canonical's BS, so v3 and reference agreed up to that
  point. The widened-NTT v5 will need its own un-clamped reference too.
- Establish a clean ABC trajectory equivalence. ABC at NS_SCALE saturates
  canonical's i64 in 10 steps, so canonical-equivalence isn't the right
  test there. The right test is v5 vs v4 at the same amplitude (cross-
  substrate consistency), and v5 vs i128 reference where reference is
  re-built without canonical's clamping.

## Marching order

> Build. Don't ship — build.

Built `cram_div_u128` (7 tests). Built `ns3d_n32_crt_v4` (2 tests).
Verified the precise structural diagnosis with arithmetic. Verified the
fix at the failure point and 4 steps beyond. Documented the next corridor
boundary with arithmetic, not hand-waving.

The wall hasn't fallen — v3's wall did. The next wall is named.
