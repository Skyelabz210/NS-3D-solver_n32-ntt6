# ERRATA & REFINEMENTS — CRT Mechanism Isolation Documents

## Date: March 16, 2026
## Applied to: CRT_MECHANISM_ISOLATION.md and CRT_MECHANISM_ISOLATION_DEFINITIVE.md

---

## ERROR 1: PLMG Rail/Void Ratio

INCORRECT (appears in both documents):
  "28% rails, 72% voids for {2,3,5,7,11}"
  "28%/72% for Safe Basis"

CORRECT:
  {2,3,5,7,11}: phi(2310)/2310 = 480/2310 = 20.8% rails / 79.2% voids
  {2,3,5,7,11,13}: phi(30030)/30030 = 5760/30030 = 19.2% rails / 80.8% voids
  {3,5,7,11} (odd only): phi(1155)/1155 = 480/1155 = 41.6% rails / 58.4% voids

EXPLANATION: The rail ratio equals the Euler totient function phi(M)/M,
which for a product of distinct primes equals product((p-1)/p). The 28%
figure does not correspond to any standard basis. The actual void ratio
is HIGHER than claimed (79-81%), meaning the error detection is BETTER
than the documents state.

IMPACT: The PLMG error detection capability is understated, not overstated.
All code (PLMGValidator in lib.rs and hydra-sieve) uses configurable bounds
and does not hardcode the incorrect ratio.

---

## REFINEMENT 1: Correction Factor Convergence

The document states the correction factor "converges toward 1." The stress
test data shows approximate but NOT strictly monotone convergence:

  Base 15:    C = 0.625
  Base 105:   C = 0.729
  Base 385:   C = 0.770
  Base 1155:  C = 0.849
  Base 2310:  C = 0.929
  Base 5005:  C = 0.889  ← dip (non-monotone)
  Base 30030: C = 0.953

The general trend is toward 1, but the path is not monotone. The dip at
base 5005 ({5,7,11,13}) may reflect the absence of primes 2 and 3 from
that base, which changes the carry structure. CONJECTURE should be stated
as "approximately converges" rather than "monotonically converges."

---

## REFINEMENT 2: CRAM Topology Count

The document states "100,000+ distinct arithmetic operations" with formula
"4! × 4^k". The correct count is (4 root ops × 4 post ops)^k = 16^k.
For k=6 (Safe Basis): 16^6 = 16,777,216 topologies. The "100,000+" is
correct but dramatically understated. The actual space is 16.7 million.

---

## REFINEMENT 3: Hydra Operator-Carry Mechanism

The original Hydra analysis (HYDRA_SIEVE_SUMMARY.pdf) states "carry sieve
does NOT eliminate individual candidates efficiently" with "0.0-0.2%
elimination rate." This was correct for PLAIN carries (mixed-radix addition)
but NOT for CRAM operator-distorted carries.

With heterogeneous CRAM operators (v3), elimination rates are:
  Twin:    79.8% (was 0.2%)
  Cousin:  75.2% (was 0.0%)
  Sexy:    56.2% (was 0.0%)
  d=30:    56.6%
  d=360:   45.5%

The original "0% residue class" finding was an honest negative about PLAIN
carries, not about the CRAM topology. The correction: plain carries map
forbidden patterns to zero residue classes, but CRAM-distorted carries
create non-uniform distributions that DO map to eliminable residue classes.

---

## VERIFIED CLAIMS (13/14)

All other claims in both documents verified against code and computation:
- Safe Basis product = 30,030 ✓
- K-Elimination O(1) exact (50,000 test values, zero errors) ✓
- Twin primes p>3 must have p ≡ 2 mod 3 (checked to 100k) ✓
- Mersenne 2^31-1 coprime to 30030 ✓
- 106 sacred cycles analysis ✓
- CRAM topology space > 100,000 (actually 16.7M) ✓
- S1 honest negative (carries = O(1) only) ✓
- Rust/Python cross-verification (all 5 families match exactly) ✓
- Zero false negatives at all tested ranges up to 1M ✓
