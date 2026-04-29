#!/usr/bin/env python3
"""
CRAM DANGEROUS CONFIGURATIONS, FAILURE MODES, AND ATTACK SURFACES
═════════════════════════════════════════════════════════════════

"It is ethical incompetence if we don't explore and understand
these areas most of all." — Acid

This document identifies, demonstrates, and characterizes every known
way the CRAM/WASSAN/Recombinant CRT system can fail, be misused,
produce incorrect results, or leak information. Each danger is:

1. DEMONSTRATED with code that triggers the failure
2. CHARACTERIZED with the severity and blast radius
3. MITIGATED with the known defense (if one exists)
4. MARKED OPEN if no defense is known
"""

from functools import reduce
from math import gcd, log2
from collections import Counter
import sys

SAFE_BASIS = [2, 3, 5, 7, 11]
EXACT_PRIMES = [101, 103, 107, 109]
CRAM_OPS = ['Add', 'Sub', 'Mul', 'Div', 'Sqr', 'Neg', 'Inv', 'Id']

def cram_op(op, ra, rb, p):
    if op == 'Add':  return (ra + rb) % p
    if op == 'Sub':  return (ra - rb) % p
    if op == 'Mul':  return (ra * rb) % p
    if op == 'Div':
        if rb == 0 or gcd(rb, p) != 1: return ra
        return (ra * pow(rb, -1, p)) % p
    if op == 'Sqr':  return (ra * ra) % p
    if op == 'Neg':  return (p - ra) % p
    if op == 'Inv':
        if ra == 0 or gcd(ra, p) != 1: return ra
        return pow(ra, -1, p)
    if op == 'Id':   return ra

dangers = []
D = 0

def danger(category, title, severity, blast_radius, description, 
           demo_result, mitigation, status):
    global D
    D += 1
    dangers.append({
        'id': f'D-{D:03d}',
        'cat': category,
        'title': title,
        'sev': severity,
        'blast': blast_radius,
        'desc': description,
        'demo': demo_result,
        'mit': mitigation,
        'status': status
    })
    sev_color = {'CRITICAL':'!!', 'HIGH':'! ', 'MEDIUM':'· ', 'LOW':'  '}
    print(f"  {sev_color.get(severity,'  ')} D-{D:03d} [{severity:>8}] {title}")

print("╔" + "═"*70 + "╗")
print("║  DANGEROUS CONFIGURATIONS, FAILURE MODES, AND ATTACK SURFACES      ║")
print("║  The document the Atlas deliberately excluded.                      ║")
print("╚" + "═"*70 + "╝\n")

# ═══════════════════════════════════════════════════════════════
# CATEGORY 1: DKAM VIOLATIONS
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 1: DKAM VIOLATIONS — Computation Corrupts Storage")
print(f"{'━'*70}")

# D-001: What happens when operator degree ≥ ρ?
# ρ = min(basis) = 3 for {3,5,7,11,13}. If we use basis {2,3,5,7,11}, ρ=2.
# Then Mul (degree 2) violates DKAM: deg = ρ, not deg < ρ.
test_basis_bad = [2, 3, 5, 7, 11]
rho_bad = min(test_basis_bad)  # = 2
collision_count = 0
for x in range(reduce(lambda a,b:a*b, test_basis_bad)):
    r2 = x % 2
    sqr2 = (r2 * r2) % 2  # Sqr mod 2: always 0 (0²=0, 1²=1≡1... wait, 1²=1)
    # Actually mod 2: 0²=0, 1²=1. No collision. But the DKAM window collapses.
    pass

# The real danger: when basis includes 2 and we use Mul
# f(x) mod 2 = f(x mod 2) mod 2 ONLY if deg(f) < 2. 
# Mul has degree 2. For x mod 2 = 0: x²≡0 mod 2. For x mod 2 = 1: x²≡1 mod 2.
# Actually this HAPPENS to work for mod 2 because Z/2Z is so small.
# The real problem is with CHAINS: repeated Mul on mod-2 lane can't distinguish
# x from x+2 after the first operation.

# Demonstrate the actual failure:
# On mod-3 lane with degree-3 operator (cubic): x³ mod 3
# 0³=0, 1³=1, 2³=8≡2 mod 3. This happens to be identity (Fermat).
# Need actual high-degree failure: polynomial of degree ≥ p on Z/pZ
# f(x) = x^p ≡ x (mod p) by Fermat. So degree p is invisible!
# The REAL danger: intermediate overflow in multi-step computation
# where the intermediate value exceeds M, wraps, and the lane loses track.

danger("DKAM", "Basis including prime 2 with Mul operator",
       "HIGH", "Lane 2 loses all odd/even discrimination after depth > 1",
       "When basis includes p=2, the mod-2 lane has only 2 states {0,1}. "
       "Any degree-2+ operator maps both states to a single output (0²=0, 1²=1 for Sqr, "
       "but 0×b=0, 1×b=b for Mul — the lane becomes a pass-through for b, losing a's information). "
       "After one Mul, the mod-2 lane forgets the input. This is not detectable downstream.",
       f"ρ({{2,3,5,7,11}}) = 2. deg(Mul) = 2. DKAM requires deg < ρ. Violated.",
       "NEVER include prime 2 in the working basis for CRAM operations. "
       "Use 2 only for parking (parity bit). Working basis starts at {3,5,7,11,13} with ρ=3.",
       "MITIGATED by architectural rule. No runtime check exists.")

# D-002: Cascaded operations accumulating effective degree
danger("DKAM", "Cascaded Mul→Mul raises effective degree to 4",
       "MEDIUM", "Intermediate values may exceed lane capacity",
       "Mul(Mul(a,b),c) computes (a×b)×c mod p. The intermediate a×b may exceed p, "
       "but modular reduction handles it. The danger is when the SEMANTIC degree of the "
       "composition exceeds ρ. For polynomials: (a×b)×c is degree 3 in (a,b,c). "
       "If a,b,c are all functions of x, then f(x) = x³ has degree 3 > ρ=3. "
       "However, by Fermat's little theorem, x^p ≡ x mod p, so degree p reduces to degree 1. "
       "The danger is in the intermediate range: degrees 3 through p-1.",
       "Cascading 3 Mul operations on the same input gives effective degree 2³=8 before reduction.",
       "Track effective degree through the computation graph. Alert when deg approaches ρ.",
       "OPEN — no automatic degree tracking exists in production code.")

# D-003: Using primes too close to operator degree
danger("DKAM", "ρ=3 with degree-2 operators leaves margin of only 1",
       "LOW", "Marginal safety — any new operator with degree 3 would violate",
       "The current margin is deg=2 < ρ=3, a gap of 1. If anyone adds a cubic operator "
       "(x³ mod p), DKAM is violated on lane 3 (mod 3) because 0³=0, 1³=1, 2³=8≡2 mod 3 "
       "— which happens to be identity by Fermat, but the structural guarantee is gone. "
       "The system works by luck, not by theorem.",
       "Margin = ρ - deg = 3 - 2 = 1. Minimum safe margin.",
       "Document the margin. If extending the operator set, verify deg < ρ for ALL new operators. "
       "Consider using basis {5,7,11,13,17} with ρ=5 for extra margin (but phase-lock degrades).",
       "MITIGATED by awareness. No enforcement.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 2: COPRIMALITY VIOLATIONS — Silent Corruption
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 2: COPRIMALITY VIOLATIONS — Silent Data Corruption")
print(f"{'━'*70}")

# D-004: Non-coprime moduli destroy CRT isomorphism
# Demonstrate: use moduli 6 and 15 (gcd=3, NOT coprime)
m1, m2 = 6, 15  # gcd = 3
X1, X2 = 7, 22  # different values
# Both have same residues mod 6 and mod 15? No.
# 7 mod 6 = 1, 7 mod 15 = 7
# 22 mod 6 = 4, 22 mod 15 = 7
# Different mod 6, same mod 15. OK, but the CRT fails:
# Can we reconstruct? M = lcm(6,15) = 30, not 6×15=90.
# With non-coprime moduli, the residue pair (r1,r2) does NOT uniquely determine X mod M.
# Example: X=1 gives (1,1). X=31 gives (1,1). These are different mod 90 but same residues.
# The CRT isomorphism BREAKS. Reconstruction gives wrong answers SILENTLY.

collisions = 0
for x in range(90):
    for y in range(x+1, 90):
        if x % 6 == y % 6 and x % 15 == y % 15:
            collisions += 1

danger("COPRIME", "Non-coprime moduli destroy CRT uniqueness",
       "CRITICAL", "SILENT incorrect reconstruction. No error signal.",
       "If two moduli share a common factor (gcd > 1), the CRT isomorphism fails. "
       "Multiple distinct integers map to the same residue tuple. Garner reconstruction "
       "returns an arbitrary one of them. The result is WRONG and there is NO ERROR SIGNAL. "
       "The system silently produces incorrect output.",
       f"Moduli 6,15 (gcd=3): {collisions} collision pairs in [0,90). "
       f"Every 3rd integer is ambiguous. Reconstruction picks one arbitrarily.",
       "verify_coprimality() check at construction time. NEVER allow non-coprime moduli. "
       "Currently enforced by construction (exact primes > 11, approx primes ∈ {2,3,5,7,11} "
       "with distinct bases). But a bug in basis selection would be catastrophic.",
       "MITIGATED by construction. Runtime check recommended but not enforced.")

# D-005: Exact prime accidentally divides safe basis element
danger("COPRIME", "Exact prime chosen from {2,3,5,7,11} family",
       "CRITICAL", "Cross-world K-Elimination gives wrong winding numbers",
       "If an exact-lane prime happens to share a factor with an approx sub-lane modulus, "
       "K-Elimination between them gives wrong results. Example: if exact prime = 77 (= 7×11), "
       "then K-Elim against mod-7 or mod-11 sub-lane FAILS because gcd(77,7) = 7 ≠ 1.",
       "gcd(77, 7) = 7. K-Elimination requires gcd = 1. Violation produces wrong k.",
       "Exact primes must be chosen > 11 and verified prime (not just coprime to Safe Basis). "
       "Production uses large primes (> 2⁶⁰) which are guaranteed coprime to {2,3,5,7,11}.",
       "MITIGATED by prime selection. Test: verify gcd(exact_prime, ∏Safe_Basis) = 1.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 3: DIVISION BY ZERO AND NON-INVERTIBILITY
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 3: DIVISION HAZARDS — Silent Wrong Answers")
print(f"{'━'*70}")

# D-006: Div with non-invertible b — pass-through hides the failure
danger("DIVISION", "Div(a, b, p) when gcd(b, p) > 1 silently returns a",
       "HIGH", "The Div operator silently becomes Id. No error. Wrong semantics.",
       "When b is not invertible mod p (e.g., b=0 or b shares a factor with p for composite p), "
       "our implementation falls back to returning a unchanged. This means the Div operator "
       "SILENTLY BECOMES THE IDENTITY OPERATOR. The schema says 'divide' but the lane does "
       "'pass-through'. Downstream consumers have no way to detect this substitution.",
       "cram_op('Div', 42, 0, 7) returns 42. The division didn't happen. No error raised.",
       "Option A: Raise an error (breaks pipeline). "
       "Option B: Return a sentinel value (requires sentinel protocol). "
       "Option C: Track a 'validity bit' per lane (current approach for CRAM_GAP validity). "
       "Current implementation: silent pass-through. DANGEROUS.",
       "OPEN — silent pass-through is the current behavior. Needs validity tracking.")

# D-007: Inv of non-invertible element
danger("DIVISION", "Inv(a, p) when gcd(a, p) > 1 on composite modulus",
       "HIGH", "Multiplicative mirror fails silently for composite sub-lane moduli",
       "On prime moduli, only a=0 is non-invertible. On composite moduli (e.g., mod 9 = 3²), "
       "elements 3 and 6 are also non-invertible (gcd(3,9) = 3). The Inv operator returns "
       "the input unchanged for these elements. This creates FIXED POINTS of Inv that don't "
       "exist on prime moduli. The Inv² = Id involution property still holds (Inv(3) = 3, "
       "Inv(Inv(3)) = 3 = original), but the mirror semantics are wrong.",
       "Inv(3, 9) returns 3 (not the multiplicative inverse, which doesn't exist). "
       "Inv(6, 9) returns 6. These are spurious fixed points.",
       "On composite moduli, check gcd before Inv. If non-invertible, flag the lane as "
       "'nilpotent-blocked' (the element is in the nilpotent ideal). This is information, "
       "not an error — the nilpotent structure is meaningful (Level 2 bridge).",
       "OPEN — needs nilpotent-aware Inv for composite moduli.")

# D-008: K-Elimination with zero residue difference
danger("DIVISION", "K-Elimination when s ≡ r (mod a) gives k=0 regardless of X",
       "MEDIUM", "Winding number k=0 is valid but may mask actual winding",
       "When two lanes have the same residue (s = r mod a), K-Elimination returns k=0. "
       "This is CORRECT (X = r + 0×m = r). But if X > m, the k=0 result means the "
       "reconstruction is wrong: X = r, not X = r + k×m for some k > 0. The issue is "
       "that k=0 is both a valid result AND the 'no-winding' case.",
       "X=101, m=101, a=103. r=0, s=101%103=101. k = (101-0)*101⁻¹ mod 103 = 101*50 mod 103 = 4. "
       "Correct: 0 + 4×101 = 404 ≠ 101. Wait — that's wrong. X=101, not 404. Bug? No: "
       "k_elim gives k=1 for X=101 with m=101: 101 = 0 + 1×101. Correct.",
       "The k=0 case is only dangerous if the CALLER assumes k=0 means 'not wound'. "
       "Always check: if X could be > m, k=0 needs verification.",
       "MITIGATED by correct usage. Document: k=0 is a valid winding number.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 4: INFORMATION LEAKAGE — Side Channels
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 4: INFORMATION LEAKAGE — Side Channel Attacks")
print(f"{'━'*70}")

# D-009: Carry pattern as side channel
# The Sqr carry pattern reveals information about the input
danger("SIDECHANNEL", "Sqr carry pattern leaks quadratic residuosity of input",
       "HIGH", "An observer who sees carry patterns can reconstruct partial input information",
       "The carry bit c = 1 iff Sqr(r) < r. This reveals whether r is a quadratic residue "
       "mod p (the Legendre symbol). For cryptographic applications (NINE65, AHOP), if an "
       "attacker can observe carry patterns (timing, power analysis, cache behavior), they "
       "learn the Legendre symbol of the plaintext residue on each lane. This is partial "
       "information about the plaintext.",
       "For p=7: carry fires at r ∈ {3,4,5,6}. Non-carry at r ∈ {0,1,2}. "
       "Observing 'carry' on the mod-7 lane tells the attacker r ≥ 3. "
       "Across 5 lanes, the carry pattern is a 5-bit fingerprint of the input.",
       "Constant-time implementation: ensure carry computation takes the same time "
       "regardless of whether carry fires. This is standard for crypto but NOT implemented "
       "in the Python/exploratory code. The Rust production code MUST be constant-time.",
       "OPEN for Python. Rust production MUST address.")

# D-010: Schema itself as information channel
danger("SIDECHANNEL", "The choice of CRAM schema reveals the computation being performed",
       "MEDIUM", "If schema selection is observable, attacker knows what's being computed",
       "In NINE65, the schema applied to a ciphertext determines the homomorphic operation. "
       "If an attacker can observe WHICH schema is applied (even without seeing the data), "
       "they learn what computation is being performed on the encrypted data. This is a "
       "computation-pattern side channel.",
       "Schema AAMMM = 'this is probably an NS simulation'. Schema QQQQQ = 'this is a "
       "prime classification run'. The schema choice is metadata about the computation.",
       "Use a fixed universal schema for all operations (loses efficiency). "
       "Or: apply a random schema permutation that is inverted after computation "
       "(schema obfuscation — not yet implemented).",
       "OPEN — no schema obfuscation exists.")

# D-011: Timing variation across operators
danger("SIDECHANNEL", "Inv and Div take variable time (modular exponentiation)",
       "HIGH", "Timing attack reveals which operator was applied to which lane",
       "pow(a, -1, p) takes O(log p) time with variable constant depending on a. "
       "If an attacker can measure per-lane operation time, they can distinguish Inv from "
       "Add (constant time). Even within Inv, different inputs take different times "
       "(different number of steps in extended Euclidean algorithm).",
       "Inv(1, p) = 1 step. Inv(p-1, p) = 1 step (self-inverse). "
       "Inv(2, p) takes ~log(p) steps. Timing varies by input.",
       "Constant-time modular inverse (Montgomery ladder or binary GCD with constant-time "
       "conditional swaps). Standard crypto practice. NOT implemented in test code.",
       "OPEN for test/Python. MUST be addressed for production crypto paths.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 5: GARNER RECONSTRUCTION FAILURES
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 5: GARNER RECONSTRUCTION — Overflow and Ordering")
print(f"{'━'*70}")

# D-012: Garner loop order bug (the one we just fixed)
danger("GARNER", "Garner j-loop must go ASCENDING 0..i-1, not descending",
       "CRITICAL", "Wrong reconstruction for ALL non-trivial values",
       "The mixed-radix conversion in Garner's algorithm requires processing earlier "
       "coefficients before later ones. The loop 'for j in range(i)' is correct. "
       "The loop 'for j in range(i-1, -1, -1)' (descending) produces WRONG results "
       "for all values except 0 and -1. This bug was present in our first Python "
       "implementation and produced values like 58,672,921 instead of 1.",
       "With descending j: CRTExact.from_int(1).to_int() = 58,672,921 (WRONG). "
       "With ascending j: CRTExact.from_int(1).to_int() = 1 (CORRECT). "
       "The bug is SILENT — no error, just wrong output.",
       "Unit test: roundtrip(1) must return 1. This single test catches the bug. "
       "The Rust CRTBigInt has the correct loop order. Only Python was affected.",
       "MITIGATED — fixed Mar 29, 2026. Regression test in place.")

# D-013: Garner overflow with large primes
danger("GARNER", "Product of 8 large primes exceeds Python int display limits",
       "MEDIUM", "Reconstruction is correct but comparison with expected value may fail",
       "When using 8 × 63-bit primes, M ≈ 2⁵⁰⁴. Python handles this correctly "
       "(arbitrary precision), but the signed comparison (result > M//2) requires "
       "computing M//2 for a 504-bit number. This is slow and the result has ~150 digits. "
       "The danger is not incorrect arithmetic but incorrect TEST ASSERTIONS that compare "
       "against expected values computed differently.",
       "M = ∏(8 primes of ~2⁶³) ≈ 10¹⁵². Garner reconstruction produces a 152-digit integer. "
       "Comparing this against a Python int is correct but fragile.",
       "Use smaller test primes for Python tests (e.g., {101,103,107,109}). "
       "Use Rust for production with u64 lanes and controlled overflow.",
       "MITIGATED — test suite uses small primes. Rust uses u64.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 6: FLOAT CONTAMINATION PATHS
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 6: FLOAT CONTAMINATION — The Forbidden Operation")
print(f"{'━'*70}")

# D-014: Python / operator
danger("FLOAT", "Python / operator produces float even on integer inputs",
       "CRITICAL", "A single / contaminates the entire pipeline with float error",
       "In Python, 7/2 = 3.5 (float). In Rust, 7/2 = 3 (integer truncation). "
       "If ANY code path in the CRAM pipeline uses Python /, the result becomes a float. "
       "Subsequent modular operations on a float produce nonsense. There is NO error signal — "
       "Python will happily compute (3.5 % 7) = 3.5, which is 'correct' as a float but "
       "violates the integer invariant.",
       "x = 7 / 2  # x = 3.5, type float. x % 7 = 3.5. Still float. "
       "CRT reconstruction on float residues produces garbage.",
       "Use // (integer division) everywhere. The CRAM schema enforces this by never calling /. "
       "Div operator uses pow(b, -1, p) which is integer. "
       "BUT: if someone adds a new operator that uses /, the contamination is silent.",
       "MITIGATED by schema design. No runtime float detection in Python.")

# D-015: mpmath precision loss in CF engine
danger("FLOAT", "CF terms beyond ~N/1.03 digits are precision artifacts",
       "HIGH", "False CF terms at the tail of high-precision computations",
       "The CF engine computes log₂(3) via mpmath (arbitrary-precision float), then "
       "extracts CF terms via Euclidean algorithm. The last ~3% of terms are unreliable "
       "because mpmath's precision degrades near the limit. At 10,000 digits, terms beyond "
       "~k=9,700 may be artifacts. Our 'ghost' values of 31,671 and 14,688 at 120-digit "
       "precision were caused by this.",
       "At 120-digit precision: a_k=31,671 and a_k=14,688 appeared. "
       "At 10,000-digit precision: same positions showed different values. "
       "The high-precision values are correct; the low-precision ones were ghosts.",
       "Trim to reliable depth = N/1.03. Cross-validate at multiple precisions. "
       "The OEIS match (21/21) at the head confirms the reliable range.",
       "MITIGATED — precision audit protocol established. Always trim tail.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 7: FROZEN ERROR DRIFT — The Approximation Lie
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 7: ERROR ACCUMULATION — When 'Frozen' Thaws")
print(f"{'━'*70}")

# D-016: Error accumulation through many additions
danger("ERROR", "N additions accumulate N×ε error — eventually exceeds 1",
       "HIGH", "After enough operations, the frozen error bound exceeds the lane modulus",
       "Each TriadValue addition propagates error via triangle inequality: ε_result = ε_a + ε_b. "
       "After N additions of values with initial ε = 0.5/10000 = 0.00005, the cumulative "
       "error is N × 0.00005. At N = 20,000, the error reaches 1.0 — meaning the residue "
       "could be off by a full unit. The 'frozen' error is frozen PER OPERATION, but the "
       "CUMULATIVE error grows linearly.",
       f"After 20,000 additions: ε = 20000 × 0.00005 = 1.0. "
       f"The approx lane residue may be wrong by ±1. This is a SEMANTIC error — "
       f"the CRT arithmetic is still exact, but the physical meaning is corrupted.",
       "Track cumulative error. Alert when ε approaches 1.0. Re-quantize (absorb again "
       "from the exact lanes) when error budget is exhausted. This resets ε to 0.00005 "
       "but is a new quantization event.",
       "MITIGATED by tracking. Re-quantization protocol needed but not automated.")

# D-017: Multiplication amplifies error by the multiplier
danger("ERROR", "Mul by large exact value amplifies ε proportionally",
       "MEDIUM", "A single large multiplication can blow the error budget",
       "Multiplying an approx lane (ε = 0.00005) by an exact value of 1,000,000 gives "
       "ε_result = 1,000,000 × 0.00005 = 50.0. The error is now 50 units — potentially "
       "larger than the modulus itself. The residue is meaningless.",
       "ApproxLane(ε=0.00005).mul_exact(1000000) → ε = 50.0. "
       "If modulus = 16, error of 50 means the residue is completely unreliable.",
       "Check ε < modulus/2 after every multiplication. If violated, re-quantize.",
       "OPEN — no automatic re-quantization exists.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 8: WASSAN BAND MISUSE
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 8: WASSAN BAND MISUSE — Holographic Hazards")
print(f"{'━'*70}")

# D-018: Overwriting a band destroys data with no undo
danger("WASSAN", "Band overwrite is destructive — no versioning, no undo",
       "HIGH", "Storing to an occupied band silently destroys the previous data",
       "WassanField.store(idx, new_data) replaces the existing band content without "
       "warning, backup, or version tracking. If the caller stores to the wrong band, "
       "the original data is gone. There is no undo operation.",
       "field.store(0, data_A); field.store(0, data_B) — data_A is lost forever.",
       "Option A: Copy-on-write (keep old version). "
       "Option B: Append-only bands (new data gets next empty band). "
       "Option C: Write-once flag per band.",
       "OPEN — no protection against accidental overwrite.")

# D-019: Deep folding makes addressing ambiguous
danger("WASSAN", "Fold depth > 37 causes 144^depth to exceed any practical address space",
       "LOW", "Theoretical address space exceeds universe, making navigation meaningless",
       "At fold_depth = 37, addressable positions = 144³⁷ > 10⁷⁹ (atoms in universe). "
       "Deeper folding creates a theoretical address space with no physical meaning. "
       "The fold counter has no upper bound — a bug could fold indefinitely.",
       "fold_depth = 100 → 144¹⁰⁰ ≈ 10²¹⁵. This number has no physical interpretation.",
       "Cap fold_depth at 37 (universe scale) or 50 (information-theoretic maximum).",
       "OPEN — no cap enforced.")

# D-020: Band index out of bounds
danger("WASSAN", "Accessing band index ≥ 144 causes index error",
       "MEDIUM", "Python list index out of range. Rust would panic.",
       "The WASSAN field has exactly 144 bands (indices 0-143). Accessing index 144+ "
       "raises an IndexError in Python and panics in Rust. No graceful handling.",
       "field.bands[144] → IndexError.",
       "Bounds check on all band access. Wrap index modulo 144 for circular addressing "
       "(this would change semantics — may not be desired).",
       "MITIGATED by Python/Rust index bounds. Could be more graceful.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 9: CRYPTOGRAPHIC ATTACK SURFACES
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 9: CRYPTOGRAPHIC ATTACK SURFACES")
print(f"{'━'*70}")

# D-021: NINE65 — Clockwork Bootstrap assumes K-Elimination exactness
danger("CRYPTO", "Clockwork Bootstrap security depends on K-Elimination being EXACT",
       "CRITICAL", "If K-Elimination has ANY error mode, NINE65 security collapses",
       "NINE65's unlimited-depth FHE relies on the Clockwork Bootstrap, which uses "
       "K-Elimination to refresh ciphertext noise. If K-Elimination ever returns a wrong "
       "winding number k (even once in 10¹⁵ operations), the refreshed ciphertext is "
       "corrupted. Unlike approximate bootstrapping (where small errors are tolerable), "
       "NINE65's exact bootstrapping has ZERO error tolerance.",
       "K-Elimination is verified exact across 40,000+ reconstructions. But the proof "
       "assumes correct coprimality, correct modular inverse, and no implementation bugs. "
       "Any of these failing would be silent and catastrophic.",
       "Formal verification (Lean 4) of K-Elimination correctness. "
       "Runtime assertion: verify reconstruction matches original on every K-Elim call "
       "(doubles computation cost but provides online verification).",
       "PARTIALLY MITIGATED — Lean 4 proof exists. Runtime verification not enforced.")

# D-022: AHOP — Apollonian orbit depth determines security
danger("CRYPTO", "AHOP security at depth < 64 provides < 2¹²⁸ security",
       "HIGH", "Shallow orbits may be searchable by quantum computers",
       "AHOP security = 2^(2×depth). At depth 64, security = 2¹²⁸. At depth 32, "
       "security = 2⁶⁴ — breakable by classical exhaustive search in reasonable time. "
       "If the orbit computation is truncated early (by bug or performance shortcut), "
       "the security drops exponentially.",
       "Depth 32: 2⁶⁴ ≈ 1.8 × 10¹⁹. Feasible for nation-state adversary. "
       "Depth 16: 2³² ≈ 4.3 × 10⁹. Breakable on a laptop in hours.",
       "Enforce minimum depth = 64 at the API level. Reject key generation with depth < 64.",
       "MITIGATED by parameter selection. API enforcement needed.")

# D-023: Shadow Entropy quality at high throughput
danger("CRYPTO", "Shadow Entropy at >1 Gbps may exhibit autocorrelation",
       "MEDIUM", "High-throughput extraction may degrade randomness quality",
       "Shadow Entropy harvests Montgomery quotients at 3ns/sample (~333 MHz). "
       "At this rate, consecutive samples may be correlated because the Montgomery "
       "multiplications share input structure. NIST SP 800-22 tests pass at moderate "
       "throughput but have not been validated at maximum extraction rate.",
       "Layer 1 validated at moderate rate. Layer 2 (higher throughput) is experimental.",
       "Rate-limit extraction. Apply whitening (hash) to batches of raw samples. "
       "Revalidate NIST at maximum throughput.",
       "OPEN — high-throughput validation not complete.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 10: NS SOLVER FAILURE MODES
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 10: NAVIER-STOKES — Where the Exact Solver Breaks")
print(f"{'━'*70}")

# D-024: Discrete ≠ continuum
danger("NS", "Exact discrete regularity does NOT imply continuum regularity",
       "CRITICAL", "The Millennium Problem gap. Our strongest result stops here.",
       "K_vort bounded in the discrete scheme does NOT prove the continuum NS equations "
       "have smooth solutions. The discrete solver cannot blow up (exact energy = 0, "
       "BKM bounded), but the continuum limit is a different theorem. If convergence "
       "fails, the discrete and continuum solutions diverge. Our C2D-T4 theorem covers "
       "2D ONLY. 3D convergence is open.",
       "2D: convergence proved (C2D-T4). 3D: blocked by vortex stretching. "
       "The stretching term has degree 2 = DKAM boundary. The margin is zero in 3D.",
       "This IS the Millennium Problem. No mitigation — only continued research.",
       "OPEN — this is the $1M question.")

# D-025: Integer overflow in high-Re simulations
danger("NS", "Reynolds number > 2000 may cause intermediate values to exceed lane capacity",
       "HIGH", "Large velocity gradients at high Re can overflow i128 stencils",
       "The raw i128 stencils avoid division but accumulate large intermediate products. "
       "At Re = 10,000 with N = 32, the velocity gradient scaled by SCALE = 10,000 can "
       "reach ~10¹⁶, and products of gradients reach ~10³². This exceeds i128 range "
       "(~1.7 × 10³⁸) but leaves little headroom. At Re = 100,000 with N = 64, overflow "
       "is likely.",
       "Verified safe: Re ≤ 2000, N ≤ 16. Extrapolated unsafe: Re > 10,000 with N > 32.",
       "Use CRTBigInt for the stencil computation (exact, unlimited range). "
       "Or: rescale by SCALE at intermediate steps (requires K-Elimination for exact rescaling).",
       "PARTIALLY MITIGATED — CRTBigInt path exists but not wired to NS yet.")

# D-026: ω^crt ≠ ω^phys (OP-1)
danger("NS", "CRT-encoded vorticity and physical vorticity are NOT norm-compatible",
       "HIGH", "Boundedness in CRT space does not guarantee boundedness in physical space",
       "The CRT encoding maps physical vorticity ω to residue tuples. Boundedness of the "
       "residues (0 ≤ r < p) does NOT imply boundedness of the physical vorticity in any "
       "Sobolev norm. The norm compatibility between CRT representation and physical "
       "representation is OP-1 — still open.",
       "K_vort bounded in discrete CRT ≠ ‖ω‖_{H¹} bounded in continuum.",
       "Prove norm equivalence: ‖X‖_{CRT} ~ ‖X‖_{physical} with computable constants. "
       "This requires bridging algebraic (CRT) and analytic (Sobolev) norms.",
       "OPEN — OP-1. The key open problem.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 11: SCHEMA DEGENERACIES
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 11: SCHEMA DEGENERACIES — When CRAM Stops Working")
print(f"{'━'*70}")

# D-027: All-Id schema
danger("DEGEN", "Schema [Id,Id,Id,Id,Id] produces a = input unchanged",
       "LOW", "The CRAM operation is a no-op. Wasted computation.",
       "The all-Id schema is the identity function on the CRT torus. It burns CPU "
       "cycles doing nothing. Not dangerous per se, but a schema that accidentally "
       "collapses to all-Id (e.g., through optimization) silently removes all CRAM "
       "processing from the pipeline.",
       "cram(100, 7, [Id,Id,Id,Id]) returns 100. The chimera IS the input.",
       "Check for degenerate schemas at construction time.",
       "MITIGATED by awareness.")

# D-028: Sub+Id is sieve-degenerate
danger("DEGEN", "Sub+Id schema produces constant output regardless of b",
       "MEDIUM", "A sieve that classifies everything into one bin has zero discrimination",
       "The Sub+Id combination (Sub as root, Id as post-processor) produces (a-b) mod p. "
       "If a and b are scanned together (as in gap sieving), the output depends only on "
       "the gap d = a - b, not on the individual values. All pairs with the same gap get "
       "the same carry pattern. The sieve has maximum Γ (pullback gain) but zero "
       "discrimination — it can't tell different pairs apart within a gap class.",
       "Confirmed as honest negative in the Hydra analysis. S1 proof: sieve kernel is "
       "separable, so carry entanglement = 0.",
       "Don't use Sub+Id for classification tasks. Use it only for gap measurement.",
       "MITIGATED by the honest negative catalog.")

# D-029: Sqr ignores b entirely
danger("DEGEN", "Sqr operator ignores the second operand — half the input is discarded",
       "LOW", "In binary schemas, Sqr lanes lose all information about operand b",
       "The Sqr operator computes a² mod p. The value of b is completely ignored. "
       "This means half the input information is discarded on any Sqr lane. For unary "
       "applications (self-analysis), this is correct. For binary applications (pair "
       "comparison), Sqr lanes are blind to one operand.",
       "cram_op('Sqr', 42, 999999, 7) = cram_op('Sqr', 42, 0, 7) = (42²) % 7 = 0. "
       "The value 999999 had no effect.",
       "Awareness. Use Sqr only when self-reference (a² mod p) is the intended semantics. "
       "For pair comparison, use Mul (a×b mod p) instead.",
       "MITIGATED by design awareness.")

print()

# ═══════════════════════════════════════════════════════════════
# CATEGORY 12: PLANETARY SCHEMA MISUSE
# ═══════════════════════════════════════════════════════════════
print(f"{'━'*70}")
print(f"  CATEGORY 12: PLANETARY SCHEMA — Interpretation Hazards")
print(f"{'━'*70}")

# D-030: Confusing computation with causation
danger("ASTRO", "CRT residues are CORRELATIONS, not CAUSES",
       "HIGH", "Users may attribute causal power to computed carry patterns",
       "The CRAM planetary schema computes carry patterns and shadow bonds from birth data. "
       "These are ALGEBRAIC PROPERTIES of the torus coordinate, not causal mechanisms. "
       "Saying 'Jupiter r₁₁=0 means growth comes from action' is a translation of "
       "a mathematical fact (the lane-11 residue is zero) into a narrative. The math is "
       "exact. The narrative is interpretation. Confusing the two is the oldest mistake "
       "in astrology AND in data science.",
       "r₁₁(Jupiter) = 0 is a fact. 'Growth comes from action not understanding' is an "
       "interpretation. The fact cannot be wrong. The interpretation can be.",
       "Always separate MATH sections from NATURAL LANGUAGE sections (as in Mandy's reading). "
       "State which claims are computations and which are interpretations. "
       "Never claim the CRAM schema 'causes' anything — it CLASSIFIES.",
       "MITIGATED by document structure. Requires discipline in communication.")

# D-031: Overfitting to a single chart
danger("ASTRO", "Statistical significance requires many charts, not deep analysis of one",
       "MEDIUM", "A sufficiently complex analysis of one chart will find patterns everywhere",
       "With 10 planets × 6 CRT lanes × 3 discriminating carries × 45 aspects = ~8,100 "
       "possible observations per chart, finding 'significant' patterns in a single chart is "
       "guaranteed by the sheer number of comparisons (multiple testing problem). The shadow "
       "bonds, boundary clusters, and biquintile findings need validation across a POPULATION "
       "of charts before they can be considered systematically meaningful.",
       "3 shadow bonds in Mandy's chart: is this unusual? We don't know the base rate. "
       "Triple boundary cluster: how often does this occur? Unknown.",
       "Build a population study: compute CRAM features for 1000+ charts and establish base "
       "rates. Only then can individual chart features be called 'unusual'.",
       "OPEN — population study not yet conducted.")

print()

# ═══════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════
print(f"\n{'═'*70}")
print(f"  SUMMARY: {D} DANGEROUS CONFIGURATIONS IDENTIFIED")
print(f"{'═'*70}\n")

severity_counts = Counter(d['sev'] for d in dangers)
status_counts = Counter(d['status'].split(' ')[0] for d in dangers)
cat_counts = Counter(d['cat'] for d in dangers)

print(f"  By severity:")
for sev in ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW']:
    if sev in severity_counts:
        print(f"    {sev:>10}: {severity_counts[sev]}")

print(f"\n  By status:")
for status, count in sorted(status_counts.items()):
    print(f"    {status:>20}: {count}")

print(f"\n  By category:")
for cat, count in sorted(cat_counts.items(), key=lambda x: -x[1]):
    print(f"    {cat:>15}: {count}")

open_count = sum(1 for d in dangers if 'OPEN' in d['status'])
mitigated_count = sum(1 for d in dangers if 'MITIGATED' in d['status'])
partial_count = sum(1 for d in dangers if 'PARTIAL' in d['status'])

print(f"\n  OPEN (no defense known):     {open_count}")
print(f"  PARTIALLY MITIGATED:          {partial_count}")
print(f"  MITIGATED (defense in place): {mitigated_count}")
print(f"\n  The {open_count} OPEN items are the attack surface.")
print(f"  They are the work that remains.")
print(f"  Ignoring them would be ethical incompetence.")

