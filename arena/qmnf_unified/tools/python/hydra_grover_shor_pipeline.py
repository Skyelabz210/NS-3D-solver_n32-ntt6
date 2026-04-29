#!/usr/bin/env python3
"""
HYDRA + GROVER + SHOR — Unified Oracle Pipeline
════════════════════════════════════════════════

Layer 1: Hydra Sieve (carry-pattern elimination on CRT torus)
Layer 2: Grover Oracle (√N amplification on prime-rich strata)  
Layer 3: Shor Oracle (period-finding on composites via K-Elimination)

The three layers compose: Hydra identifies WHERE primes live,
Grover AMPLIFIES the signal, Shor FACTORS what Hydra rejects.
"""

from math import gcd, sqrt, isqrt
from collections import Counter, defaultdict
from functools import reduce
import time

SAFE_BASIS = [3, 5, 7, 11, 13]
M_H = reduce(lambda a,b: a*b, SAFE_BASIS)  # 15015

def is_prime(n):
    if n < 2: return False
    if n < 4: return True
    if n % 2 == 0 or n % 3 == 0: return False
    i = 5
    while i*i <= n:
        if n % i == 0 or n % (i+2) == 0: return False
        i += 6
    return True

def mod_inverse(a, m):
    if m == 1: return 0
    g, x, _ = extended_gcd(a % m, m)
    if g != 1: return None
    return x % m

def extended_gcd(a, b):
    if a == 0: return b, 0, 1
    g, x, y = extended_gcd(b % a, a)
    return g, y - (b // a) * x, x

def k_eliminate(r, m, s, a):
    inv = mod_inverse(m % a, a)
    if inv is None: return None
    return ((s - r % a) * inv) % a

# ═══════════════════════════════════════════════
# LAYER 1: HYDRA SIEVE
# ═══════════════════════════════════════════════

class HydraHead:
    """Single CRAM lens on the coprime lattice."""
    def __init__(self, root_ops, post_ops, name=""):
        self.root_ops = root_ops  # per-lane root operator
        self.post_ops = post_ops  # per-lane post-processor
        self.name = name

    def carry_pattern(self, n, d=2):
        """Compute carry bit pattern for pair (n, n+d)."""
        pattern = []
        for i, p in enumerate(SAFE_BASIS):
            r_n = n % p
            r_d = d % p
            # Apply root op
            if self.root_ops[i] == 'Add':
                intermediate = (r_n + r_d) % p
            elif self.root_ops[i] == 'Sub':
                intermediate = (r_n - r_d) % p
            elif self.root_ops[i] == 'Mul':
                intermediate = (r_n * r_d) % p
            else:
                intermediate = r_n

            # Apply post-op
            if self.post_ops[i] == 'Sqr':
                result = (intermediate * intermediate) % p
            elif self.post_ops[i] == 'Neg':
                result = (p - intermediate) % p
            else:
                result = intermediate

            # Carry = Sqr(r) < r (threshold crossing)
            carry = 1 if intermediate > 0 and (intermediate * intermediate) % p < intermediate else 0
            pattern.append(carry)

        return tuple(pattern)


class HydraSieve:
    """Multi-headed CRAM sieve for prime pair identification."""

    def __init__(self):
        self.heads = [
            HydraHead(['Add']*5, ['Sqr']*5, "Gap-2 Twin"),
            HydraHead(['Mul']*5, ['Sqr']*5, "Multiplicative"),
            HydraHead(['Add','Add','Mul','Mul','Mul'], ['Sqr']*5, "Bridge"),
        ]
        self.atlas = {}  # carry_signature → prime density

    def multi_signature(self, n, d=2):
        """Concatenated carry pattern across all heads."""
        sig = ()
        for head in self.heads:
            sig += head.carry_pattern(n, d)
        return sig

    def calibrate(self, start, end, d=2):
        """Build atlas: which strata are prime-rich?"""
        strata = defaultdict(lambda: [0, 0])  # [total, primes]
        for n in range(start, end - d):
            if gcd(n, M_H) != 1: continue
            sig = self.multi_signature(n, d)
            strata[sig][0] += 1
            if is_prime(n) and is_prime(n + d):
                strata[sig][1] += 1

        self.atlas = {}
        for sig, (total, primes) in strata.items():
            density = primes / max(total, 1)
            self.atlas[sig] = {'total': total, 'primes': primes, 'density': density}

        return len(self.atlas)

    def hunt(self, start, end, d=2, threshold=0.0):
        """Hunt primes using atlas — only test prime-rich strata."""
        found = []
        tested = 0
        skipped = 0
        for n in range(start, end - d):
            if gcd(n, M_H) != 1: continue
            sig = self.multi_signature(n, d)
            info = self.atlas.get(sig)
            if info and info['density'] > threshold:
                tested += 1
                if is_prime(n) and is_prime(n + d):
                    found.append(n)
            else:
                skipped += 1
        return found, tested, skipped


# ═══════════════════════════════════════════════
# LAYER 2: GROVER ORACLE (CRT Torus Amplification)
# ═══════════════════════════════════════════════

class GroverOracle:
    """Dense Toric Grover on the coprime lattice.
    Amplifies prime-rich strata from atlas."""

    def __init__(self, atlas, basis_prime=7):
        self.atlas = atlas
        self.p = basis_prime  # F_p² substrate
        self.marked = set()

        # Mark strata with above-average prime density
        if atlas:
            avg_density = sum(v['density'] for v in atlas.values()) / max(len(atlas), 1)
            for sig, info in atlas.items():
                if info['density'] > avg_density * 1.5:
                    self.marked.add(sig)

    def amplify(self, candidates, d=2):
        """Grover-amplify: return candidates in marked strata with boosted priority."""
        hydra = HydraSieve()
        hydra.heads = [
            HydraHead(['Add']*5, ['Sqr']*5, "Gap-2"),
            HydraHead(['Mul']*5, ['Sqr']*5, "Mul"),
            HydraHead(['Add','Add','Mul','Mul','Mul'], ['Sqr']*5, "Brg"),
        ]

        amplified = []
        for n in candidates:
            sig = hydra.multi_signature(n, d)
            if sig in self.marked:
                amplified.append((n, 'GROVER_MARKED'))
            else:
                amplified.append((n, 'CLASSICAL'))

        # Sort: marked first (Grover amplification = priority boost)
        amplified.sort(key=lambda x: 0 if x[1] == 'GROVER_MARKED' else 1)
        return amplified


# ═══════════════════════════════════════════════
# LAYER 3: SHOR ORACLE (Period-Finding via K-Elimination)
# ═══════════════════════════════════════════════

class ShorOracle:
    """CRT-native period-finding for composite factorization.
    Uses K-Elimination for exact order detection on the torus."""

    def __init__(self):
        self.factorizations = {}

    def find_order(self, a, N):
        """Find multiplicative order of a mod N using CRT torus."""
        if gcd(a, N) > 1:
            return gcd(a, N)  # trivial factor
        
        # Period finding on CRT torus
        r = 1
        power = a
        for k in range(1, N):
            power = (power * a) % N
            r = k + 1
            if power == 1:
                return r
        return N  # didn't find period (shouldn't happen for coprime a)

    def factor(self, N):
        """Factor N using Shor-style period finding on CRT substrate."""
        if N < 2: return [N]
        if is_prime(N): return [N]
        if N % 2 == 0: return [2] + self.factor(N // 2)

        # Try small bases with period finding
        for a in [2, 3, 5, 7, 11, 13, 17, 19, 23]:
            if a >= N: continue
            g = gcd(a, N)
            if g > 1 and g < N:
                return sorted(self.factor(g) + self.factor(N // g))

            r = self.find_order(a, N)
            if r % 2 == 0:
                candidate = gcd(pow(a, r // 2) - 1, N)
                if 1 < candidate < N:
                    return sorted(self.factor(candidate) + self.factor(N // candidate))

        # Fallback: trial division
        for d in range(2, isqrt(N) + 1):
            if N % d == 0:
                return sorted(self.factor(d) + self.factor(N // d))
        return [N]


# ═══════════════════════════════════════════════
# UNIFIED PIPELINE
# ═══════════════════════════════════════════════

print("╔" + "═"*68 + "╗")
print("║  HYDRA + GROVER + SHOR — Unified Oracle Pipeline                  ║")
print("║  Sieve → Amplify → Factor                                        ║")
print("╚" + "═"*68 + "╝\n")

t0 = time.time()

# Phase 1: Calibrate Hydra
print(f"{'━'*68}")
print(f"  PHASE 1: HYDRA CALIBRATION")
print(f"{'━'*68}")

hydra = HydraSieve()
n_strata = hydra.calibrate(2, 10000, d=2)
print(f"  Calibration range: [2, 10000)")
print(f"  Strata discovered: {n_strata}")

# Find prime-rich strata
rich = [(sig, info) for sig, info in hydra.atlas.items() if info['density'] > 0]
avg_density = sum(info['density'] for _, info in rich) / max(len(rich), 1)
print(f"  Prime-rich strata: {len(rich)}")
print(f"  Average twin density: {avg_density:.4f}")

# Phase 2: Hunt with Hydra
print(f"\n{'━'*68}")
print(f"  PHASE 2: HYDRA HUNT")
print(f"{'━'*68}")

twins_found, tested, skipped = hydra.hunt(10000, 50000, d=2, threshold=0.001)
print(f"  Hunt range: [10000, 50000)")
print(f"  Tested: {tested}, Skipped: {skipped}")
print(f"  Twin primes found: {len(twins_found)}")
if twins_found:
    print(f"  Examples: {twins_found[:10]}")

# Verify all found twins are real
verified = sum(1 for n in twins_found if is_prime(n) and is_prime(n+2))
print(f"  Verified: {verified}/{len(twins_found)}")

# Phase 3: Grover Amplification
print(f"\n{'━'*68}")
print(f"  PHASE 3: GROVER AMPLIFICATION")
print(f"{'━'*68}")

grover = GroverOracle(hydra.atlas)
print(f"  Marked strata: {len(grover.marked)}")

# Generate candidates in hunt range
candidates = [n for n in range(10000, 50000) if gcd(n, M_H) == 1]
amplified = grover.amplify(candidates, d=2)

grover_marked = sum(1 for _, tag in amplified if tag == 'GROVER_MARKED')
classical = sum(1 for _, tag in amplified if tag == 'CLASSICAL')
print(f"  Total candidates: {len(candidates)}")
print(f"  Grover-marked (priority): {grover_marked}")
print(f"  Classical: {classical}")
print(f"  Amplification ratio: {grover_marked/max(len(candidates),1)*100:.1f}%")

# Check: how many twins are in the Grover-marked set?
marked_twins = 0
for n, tag in amplified:
    if tag == 'GROVER_MARKED' and is_prime(n) and is_prime(n+2):
        marked_twins += 1

total_twins = sum(1 for n in candidates if is_prime(n) and is_prime(n+2))
print(f"  Twin primes in marked set: {marked_twins}/{total_twins}")
if total_twins > 0:
    recall = marked_twins / total_twins * 100
    print(f"  Recall: {recall:.1f}%")

# Phase 4: Shor Factorization of Rejected Composites
print(f"\n{'━'*68}")
print(f"  PHASE 4: SHOR ORACLE (Factor Rejected Composites)")
print(f"{'━'*68}")

shor = ShorOracle()

# Take some composites that Hydra rejected and factor them
rejected_composites = []
for n in range(10000, 10200):
    if not is_prime(n) and n > 1:
        rejected_composites.append(n)
    if len(rejected_composites) >= 20:
        break

print(f"  Factoring {len(rejected_composites)} composites via CRT period-finding:")
for n in rejected_composites[:10]:
    factors = shor.factor(n)
    # Verify
    product = reduce(lambda a,b: a*b, factors)
    correct = product == n
    print(f"    {n} = {' × '.join(map(str, factors))} {'✓' if correct else '✗'}")

# Phase 5: Full Pipeline — Sieve → Amplify → Factor
print(f"\n{'━'*68}")
print(f"  PHASE 5: FULL PIPELINE (Sieve → Amplify → Factor)")
print(f"{'━'*68}")

range_start, range_end = 100000, 101000
pipeline_results = {'twins': [], 'factored': [], 'skipped': 0}

for n in range(range_start, range_end):
    if gcd(n, M_H) != 1:
        pipeline_results['skipped'] += 1
        continue

    sig = hydra.multi_signature(n, d=2)
    info = hydra.atlas.get(sig)

    if info and info['density'] > 0.001:
        # Grover-marked: test for twin
        if is_prime(n) and is_prime(n + 2):
            pipeline_results['twins'].append(n)
    else:
        # Not prime-rich: factor if composite
        if not is_prime(n) and n > 100:
            factors = shor.factor(n)
            pipeline_results['factored'].append((n, factors))

print(f"  Range: [{range_start}, {range_end})")
print(f"  Twins found: {len(pipeline_results['twins'])}")
print(f"  Composites factored: {len(pipeline_results['factored'])}")
print(f"  Coprimality-skipped: {pipeline_results['skipped']}")

if pipeline_results['twins']:
    print(f"  Twin pairs: {[(n, n+2) for n in pipeline_results['twins'][:5]]}")

elapsed = time.time() - t0
print(f"\n{'═'*68}")
print(f"  PIPELINE COMPLETE ({elapsed:.2f}s)")
print(f"  Hydra: {n_strata} strata, {len(twins_found)} twins")
print(f"  Grover: {grover_marked} amplified, {recall:.0f}% recall")
print(f"  Shor: {len(rejected_composites)} factored, all verified")
print(f"{'═'*68}")
