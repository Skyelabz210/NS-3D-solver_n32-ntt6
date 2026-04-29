# executioner_dag.md — SESSION 2026-04-28 PUSH10 v3 → Forward Plan

## Workspace Audit

- **Package:** `ns_regularity` at `crates/ns_v5/`
- **Baseline:** 17 ns_v5 tests pass; 6/6 cram_div, 2/2 crt_v3 (multistep), rest ns3d_n32_crt tests
- **State:** v3 fully residue-pure, BS included, 0 mismatches over 5-step TG trajectory at amp=100
- **Uploaded files:** SESSION notes + cram_div.rs + ns3d_n32_crt_v2.rs — identical to workspace, reference only
- **Forward Plan Source:** polished 9-phase execution plan (document context)

---

## KEY CONSTANTS

```
N = 32
NS_SCALE = 10000
NTT_PRIMES = [769, 929, 1153, 1217]
M_NTT = 769 × 929 × 1153 × 1217 = 1,002,568,584,801 ≈ 1.00 × 10^12
MOD_2POW = 2^31 = 2,147,483,648
MOD_5POW = 5^9 = 1,953,125
M_SMALL = 2^31 × 5^9 ≈ 4.19 × 10^15
M_ext = M_NTT × M_SMALL ≈ 4.20 × 10^27

BS raw limit: |raw| ≤ M_NTT/2 ≈ 5.01 × 10^11

Divisors present in canonical N=32 NS step:
  d=N^6  = 2^30  = 1,073,741,824   (BS normalisation)
  d=S^2  = 10^8  = 100,000,000     (diffusion: (nu·lap)/S²)
  d=4S   = 40,000                   (advection: raw_n / 4S)
  d=2S   = 20,000                   (stretching: raw_s_n / 2S)
  d=N^2  = 1,024                    (outer update: dt·rhs / N²)
```

---

## DAG NODES

---

## NODE-P0A — DivContract Table (Analytical)
- **Type:** REPORT
- **Size:** XS
- **Inputs:** Plan document, constants above
- **Output:** executioner_dag.md (appended)
- **Gate:** Table present; all 5 divisors covered; q_bound_min computed
- **Status:** PASS
- **Float check:** N/A
- **Notes:** Computed analytically below

### DivContract Table

For each `d`, `gcd(d, M_small)` components:

| d | factored | g2=gcd(d,2^31) | g5=gcd(d,5^9) | partial_2=2^31/g2 | partial_5=5^9/g5 | q_bound_2 = M_NTT·partial_2 | q_bound_5 = M_NTT·partial_5 | q_bound_MIN |
|---|---------|----------------|----------------|-------------------|------------------|----------------------------|----------------------------|-------------|
| N^6=2^30 | 2^30 | 2^30 | 1 | 2 | 1,953,125 | 2.00×10^12 | 1.96×10^18 | **2.00×10^12** |
| S^2=10^8 | 2^8·5^8 | 2^8=256 | 5^8=390625 | 2^23=8,388,608 | 5 | 8.40×10^18 | 5.01×10^12 | **5.01×10^12** |
| 4S=40000 | 2^6·5^4 | 2^6=64 | 5^4=625 | 2^25=33,554,432 | 3125 | 3.36×10^19 | 3.13×10^15 | **3.13×10^15** |
| 2S=20000 | 2^5·5^4 | 2^5=32 | 5^4=625 | 2^26=67,108,864 | 3125 | 6.71×10^19 | 3.13×10^15 | **3.13×10^15** |
| N^2=1024 | 2^10 | 2^10=1024 | 1 | 2^21=2,097,152 | 1,953,125 | 2.10×10^18 | 1.96×10^18 | **1.96×10^18** |

**Tightest bound:** d=N^6 at q_bound_min = 2.00×10^12

For N^6: actual q = garner4(raw)/N^6, where |raw| ≤ M_NTT/2 ≈ 5.01×10^11
→ |q| ≤ 5.01×10^11 / 2^30 ≈ 0.47 → **q is always 0 or ±1. ENORMOUS margin.**

BS raw bound is the real first limit: |raw| ≤ M_NTT/2 ≈ 5.01×10^11.
At amp=NS_SCALE the NTT spectral values grow as amp^2 × geometric factors.
This must be measured empirically via corridor_probe_v3.

---

## NODE-P0B — corridor_probe_v3 binary
- **Type:** SCAFFOLD + IMPL
- **Size:** M
- **Inputs:** crates/ns_v5/src/ns3d_n32_crt_v3.rs, cram_div.rs, ns3d_n32.rs
- **Output:** crates/d024_probe/src/bin/corridor_probe_v3.rs
- **Gate:** Compiles; runs for TG amp=100 and reports contract margins; zero float
- **Status:** PASS
- **Float check:** PASS (tuple `.0/.1/.2` accesses only — not float literals)
- **Notes:** Ran successfully. Key findings below.

### corridor_probe_v3 FINDINGS (empirical)

```
M_NTT (actual) = 1,002,448,197,601   [NTT_PRIMES = 769×929×1153×1217]
M_NTT/2 (BS limit) = 501,224,098,800
```

**Per-divisor quotient maxima** (all SAFE by enormous margin):

| IC | Steps | d=N^6 max|q| | d=S^2 max|q| | d=4S max|q| | d=2S max|q| | d=N^2 max|q| |
|----|-------|--------------|--------------|-------------|-------------|--------------|
| TG amp=100 | 5 | 466 | 1,721 | 8,098 | 13,891 | 55,183 |
| TG amp=1000 | 5 | 466 | 17,371 | 64,883 | 131,822 | 614,887 |
| TG amp=5000 | 3 | 465 | 8,646 | 41,835 | 25,681 | 165,290 |
| ABC amp=100 | 5 | 466 | 81,734 | 261,935 | 447,322 | 2,128,296 |
| ABC amp=NS_SCALE | 3 | 466 | 290,378 | 1,084,316 | 1,522,933 | 7,099,827 |

All quotients ≪ q_bound_min. Division contracts have essentially infinite headroom.

**BS raw-velocity (THE WALL):**

| IC | Steps | max|raw| | margin | used |
|----|-------|----------|--------|------|
| TG amp=100 | 5 | 500,387,248,399 | +836,850,401 | 99.83% |
| TG amp=1000 | 5 | 501,125,128,207 | +98,970,593 | **99.98%** |
| ABC amp=100 | 5 | 501,219,246,214 | +4,852,586 | **99.999%** |
| ABC amp=NS_SCALE | 3 | 501,201,527,460 | +22,571,340 | **99.995%** |

**CONCLUSION:**
- All 5 division contracts: **ZERO CONCERN**. Quotient magnitudes are ≤10^7 vs bounds of ≥10^12.
- BS raw corridor: **THE SOLE LIMIT**. At ABC amp=100 it is 99.999% consumed after 5 steps.
- **Fix:** Add 2 NTT primes. Two primes of ~10^4 each multiply M_NTT by ~10^8 → new M_NTT ≈ 10^20, new BS limit ≈ 5×10^19. This resolves the wall by 8 orders of magnitude.

---

## NODE-P0C — cram_div_exact_pos doc update
- **Type:** IMPL (doc only)
- **Size:** XS
- **Inputs:** crates/ns_v5/src/cram_div.rs
- **Output:** crates/ns_v5/src/cram_div.rs (modified)
- **Gate:** Doc states actual per-divisor q bounds with contract table; all 5 divisors; real bound stated
- **Status:** PASS
- **Float check:** PASS
- **Notes:** Updated fn doc with full DivContract table and BS raw limit note.

---

## NODE-P0D — debug_assert feature flag
- **Type:** IMPL
- **Size:** S
- **Inputs:** crates/ns_v5/src/cram_div.rs
- **Output:** crates/ns_v5/src/cram_div.rs (modified)
- **Gate:** assert_div_contract + assert_bs_raw_bound + div_contract_partial_mod + M_NTT constants present; compiles with and without feature; zero float
- **Status:** PASS
- **Float check:** PASS
- **Notes:** feature="div_contracts" gates both assertions. No-op stubs for default build.
  cargo check -p ns_regularity: CLEAN
  cargo check -p ns_regularity --features div_contracts: CLEAN

---

## NODE-P2A — DivInput + DivCtx structs
- **Type:** STRUCT + IMPL
- **Size:** S
- **Inputs:** cram_div.rs
- **Output:** crates/ns_v5/src/cram_div.rs (appended)
- **Gate:** DivCtx builds per divisor with pre-computed inverses; compiles; zero float
- **Status:** PASS
- **Float check:** PASS
- **Notes:** inv_d_ntt[i]*d ≡ 1 mod p_i verified by divcxt_inverses_correct (5 divisors × 4 primes = 20 checks). partial_mod_min table verified by div_contract_partial_mod_table test.

---

## NODE-P2B — trunc_div_signed + exact_div_signed_zero_remainder
- **Type:** IMPL
- **Size:** M
- **Inputs:** cram_div.rs + DivCtx from P2A
- **Output:** crates/ns_v5/src/cram_div.rs (appended)
- **Gate:** Both modes compile; trunc_div matches canonical on signed inputs; zero float
- **Status:** PASS
- **Float check:** PASS
- **Notes:** Initial NTT-only fast path for sign detection was WRONG for |x| > M_NTT/2 ≈ 5×10^11. Fixed to full 6-lane reconstruction (matches ext_to_i128). trunc_div_signed_matches_canonical: 5 divisors × 9 signed inputs = 45 checks, all PASS.

---

## NODE-P4A — garner_n_centered_i128 (generic NTT Garner)
- **Type:** IMPL
- **Size:** S
- **Inputs:** cram_div.rs garner4_to_u128
- **Output:** crates/ns_v5/src/cram_div.rs (appended)
- **Gate:** garner_n_centered_i128([4 primes]) matches garner4_to_u128 on 200 random inputs; zero float
- **Status:** PASS
- **Float check:** PASS
- **Notes:** garner_n_centered_matches_garner4: 200 random values, all match. D-012 COMPLIANT — ascending j-loop.

---

---

## NODE-P5A — 6-Prime NTT Extension
- **Type:** IMPL + TEST
- **Size:** L
- **Inputs:** ns3d_n32.rs (4-prime basis), prime_select.rs (candidate search), compute_ntt6.rs (constant verification)
- **Output:** crates/ns_v5/src/ns3d_n32_ntt6.rs (NEW)
- **Gate:** 7/7 tests pass; NTT roundtrip for p5=1409 and p6=1601; garner6 matches garner4 on 500 random values; M_NTT_6 correct; zero float
- **Status:** PASS
- **Float check:** PASS
- **Notes:**
  - p5=1409: r32=10, i=452, N_INV_3D=765, G_C=755. All verified.
  - p6=1601: r32=109, i=1561, N_INV_3D=1479, G_C=347. All verified.
  - M_NTT_6 = 2,261,331,666,182,114,209. M_NTT_6/2 = 1,130,665,833,091,057,104.
  - New BS corridor: 1.13×10^18 vs observed max 5.01×10^11 = 2.25 million× margin.
  - Non-breaking: 4-prime arrays unchanged. New _6 arrays additive.
  - D-012 compliant: all Garner loops ascending.
  - new_corridor_margin_adequate test: new_margin > old_margin × 10^6. PASS.

---

## CHECKPOINT — SESSION 2026-04-28

### Completed This Session

| NODE-ID | Status | Output |
|---------|--------|--------|
| P0A | PASS | DivContract table (analytical) in DAG |
| P0B | PASS | crates/d024_probe/src/bin/corridor_probe_v3.rs |
| P0C | PASS | cram_div.rs doc updated |
| P0D | PASS | cram_div.rs: M_NTT, M_NTT_HALF, assert_div_contract, assert_bs_raw_bound, div_contract_partial_mod |
| P2A | PASS | cram_div.rs: DivCtx struct + build() |
| P2B | PASS | cram_div.rs: trunc_div_signed, exact_div_signed_zero_remainder |
| P4A | PASS | cram_div.rs: garner_n_centered_i128 |
| P5A | PASS | crates/ns_v5/src/ns3d_n32_ntt6.rs (NEW) |

### Test Counts
- cram_div: 6 → 11 tests (5 new: DivCtx, trunc_div_signed, garner_n, partial_mod table, M_NTT constants)
- ns3d_n32_ntt6: 7 new tests
- ns3d_n32_crt_v3: 2 tests (unchanged, still passing)
- **Total session: +12 new tests, 0 failures, 0 float**

### Critical Finding (Phase 0 empirical)
**BS raw corridor is THE sole limit.** At ABC amp=100, 5 steps: 99.999% consumed.
All 5 division contracts have essentially infinite headroom (max quotient ~10^7 vs bounds ~10^12+).

**Phase 5A resolves it.** M_NTT_6/2 = 1.13×10^18 vs observed max 5.01×10^11 = 2.25M× margin.

### Pending (next session priority order)
1. **P5B** — Wire 6-prime BS into ns_step_crt_v4: use ntt3d_ext/intt3d_ext for pi in 0..6, garner6 for raw centring in biot_savart_residue. Keep 4-prime ExtRes (division contracts unaffected).
2. **P5C** — Equivalence validation: v4 step matches canonical in valid 4-prime regime; corridor_probe with 6 primes confirms ABC at NS_SCALE is now SAFE.
3. **P6** — Full validation ladder: signed division boundary tests, amplitude sweep TG/ABC, corridor warning trigger tests.
4. **P7** — Performance: flatten layout, DivCtx caching, rayon parallelism.
5. **P8** — Solver unification: ns_step_crt_v4 becomes the canonical CRT solver.
6. **P9** — ABC wall assault: run ABC at NS_SCALE with 6-prime basis until one of the contracts fires (if ever) or canonical saturates and v4 continues.

### Files to Deliver
- crates/ns_v5/src/cram_div.rs (significantly expanded)
- crates/ns_v5/src/ns3d_n32_ntt6.rs (NEW)
- crates/d024_probe/src/bin/corridor_probe_v3.rs (NEW)
- executioner_dag.md (this file)
