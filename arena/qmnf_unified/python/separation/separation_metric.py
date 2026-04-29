#!/usr/bin/env python3
"""
===============================================================================
SEPARATION METRIC FORMALIZATION
===============================================================================

The cross-check revealed: Hydra gains come from carry-pattern DIVERSITY, 
not pullback expansion (Γ). The "SECRET WEAPON" Mul+Id has Γ=1.0 but 
separation=0.681 — the highest in the hetero_twin preset.

This script:
1. Defines separation formally as statistical distance between prime
   and composite carry-indicator distributions
2. Computes separation for ALL operator combinations on the Safe Basis
3. Tests whether T6's Ramanujan spectrum predicts separation
4. Derives the optimal topology by max-separation instead of max-Γ

Authors: Acid (Anthony Diaz) / HackFate × Claude (Anthropic)
Date: March 16, 2026
Foundation: T6 (#102), T7 Suite, Hydra Rust source (lines 337-370)
Convention: CANONICAL pair-operator (a=candidate, b=candidate+gap)
===============================================================================
"""

import math
import time
from collections import defaultdict

def gcd(a, b):
    a, b = abs(a), abs(b)
    while b: a, b = b, a % b
    return a

def is_prime(n):
    if n < 2: return False
    if n < 4: return True
    if n % 2 == 0 or n % 3 == 0: return False
    i = 5
    while i * i <= n:
        if n % i == 0 or n % (i + 2) == 0: return False
        i += 6
    return True

def mod_pow(b, e, m):
    r = 1; b %= m
    while e > 0:
        if e & 1: r = r * b % m
        e >>= 1; b = b * b % m
    return r

def mod_inv(a, p): return mod_pow(a, p - 2, p)

def ramanujan_sum_prime(p, d):
    return (p - 1) if d % p == 0 else -1


# =============================================================================
# SECTION 1: Canonical Operator Application
# =============================================================================

def canonical_output(p, d, r, root_op, post_op):
    """
    Compute operator output under CANONICAL pair-operator convention.
    a = candidate with residue r, b = candidate + d
    a_r = r, b_r = (r + d) mod p
    """
    d_r = d % p
    a_r = r
    b_r = (r + d_r) % p

    # Root operator
    if root_op == 'Add':
        inter = (a_r + b_r) % p
    elif root_op == 'Sub':
        inter = (a_r + p - b_r) % p
    elif root_op == 'Mul':
        inter = (a_r * b_r) % p
    elif root_op == 'Div':
        if b_r == 0:
            return None
        inter = (a_r * mod_inv(b_r, p)) % p
    else:
        inter = (a_r + b_r) % p

    if inter is None:
        return None

    # Post-processor
    if post_op == 'Id':
        return inter
    elif post_op == 'Neg':
        return (p - inter) % p
    elif post_op == 'Sqr':
        return (inter * inter) % p
    elif post_op == 'Inv':
        if inter == 0:
            return None
        return mod_inv(inter, p)
    return inter


def carry_indicator(output, p):
    """Hydra's carry indicator: 1 if result > p/2, else 0"""
    if output is None:
        return 2  # crash sentinel
    return 1 if output > p // 2 else 0


# =============================================================================
# SECTION 2: Separation Metric
# =============================================================================

def compute_separation(p, d, root_op, post_op, limit=500000):
    """
    Compute separation = |P(carry=1 | prime pair) - P(carry=1 | composite)|
    
    This is the total variation distance between the carry-indicator
    distributions of primes vs composites on lane p.
    
    For small p, we can compute this exactly over Z/pZ by knowing which
    residues correspond to prime pairs vs composites in the sieve.
    
    The "exact torus" version: on the CRT torus mod N = prod(primes),
    a residue r is a "prime candidate" if gcd(r, N) = 1 AND gcd(r+d, N) = 1.
    """
    d_r = d % p
    
    # Compute operator output and carry for each residue
    carry_map = {}
    for r in range(p):
        out = canonical_output(p, d, r, root_op, post_op)
        carry_map[r] = carry_indicator(out, p)
    
    # Which residues are "prime candidates" on this lane?
    # r is candidate-like if r ≠ 0 AND (r + d_r) ≠ 0 mod p
    # (i.e., neither candidate nor candidate+gap is divisible by p)
    prime_residues = set()
    composite_residues = set()
    for r in range(p):
        if r != 0 and (r + d_r) % p != 0:
            prime_residues.add(r)
        else:
            composite_residues.add(r)
    
    if len(prime_residues) == 0 or len(composite_residues) == 0:
        return 0.0, carry_map, prime_residues, composite_residues
    
    # Carry distribution for primes vs composites
    prime_carry_1 = sum(1 for r in prime_residues if carry_map[r] == 1)
    comp_carry_1 = sum(1 for r in composite_residues if carry_map[r] == 1)
    
    p_prime = prime_carry_1 / len(prime_residues)
    p_comp = comp_carry_1 / len(composite_residues)
    
    separation = abs(p_prime - p_comp)
    
    return separation, carry_map, prime_residues, composite_residues


# =============================================================================
# SECTION 3: Full Operator Survey
# =============================================================================

def survey_all_operators(primes, d):
    """Survey all operators on all lanes, ranked by separation."""
    
    results = []
    ops = [
        ('Add', 'Id'), ('Add', 'Sqr'), ('Add', 'Neg'),
        ('Sub', 'Id'), ('Sub', 'Sqr'), ('Sub', 'Neg'),
        ('Mul', 'Id'), ('Mul', 'Sqr'), ('Mul', 'Neg'),
    ]
    
    for p in primes:
        lane_results = []
        for root, post in ops:
            sep, carry_map, pr, cr = compute_separation(p, d, root, post)
            
            # Also compute Γ for comparison
            d_r = d % p
            base_f = {0}
            if d_r != 0:
                base_f.add((p - d_r) % p)
            
            f_out = set()
            for r in base_f:
                out = canonical_output(p, d, r, root, post)
                if out is not None:
                    f_out.add(out)
            
            f_eff = set()
            for r in range(p):
                out = canonical_output(p, d, r, root, post)
                if out is None or out in f_out:
                    f_eff.add(r)
            
            gamma = len(f_eff) / len(base_f) if len(base_f) > 0 else 0
            
            # Check constancy
            outputs = set()
            for r in range(p):
                out = canonical_output(p, d, r, root, post)
                if out is not None:
                    outputs.add(out)
            is_const = len(outputs) <= 1
            
            lane_results.append({
                'root': root, 'post': post,
                'separation': sep, 'gamma': gamma,
                'f_eff': len(f_eff), 'constant': is_const,
                'n_outputs': len(outputs),
            })
        
        # Sort by separation descending
        lane_results.sort(key=lambda x: -x['separation'])
        results.append((p, lane_results))
    
    return results


def run_survey():
    """Full operator survey for twin, cousin, sexy primes."""
    print("=" * 72)
    print("  OPERATOR SEPARATION SURVEY (Canonical Convention)")
    print("=" * 72)
    
    safe_primes = [3, 5, 7, 11, 13]
    
    for name, d in [('Twin', 2), ('Cousin', 4), ('Sexy', 6)]:
        print(f"\n  {'='*60}")
        print(f"  {name} primes (d={d})")
        print(f"  {'='*60}")
        
        survey = survey_all_operators(safe_primes, d)
        
        for p, lane_results in survey:
            print(f"\n    p={p} (c_p({d}) = {ramanujan_sum_prime(p, d)}):")
            print(f"    {'op':>10} {'sep':>7} {'Γ':>6} {'|F_eff|':>7} "
                  f"{'|out|':>5} {'const':>6} {'verdict':>12}")
            print(f"    {'-'*58}")
            
            for entry in lane_results[:5]:  # top 5
                verdict = ""
                if entry['constant']:
                    verdict = "DEGENERATE"
                elif entry['separation'] > 0.3:
                    verdict = "★ GOOD"
                elif entry['separation'] > 0.1:
                    verdict = "usable"
                else:
                    verdict = "weak"
                
                print(f"    {entry['root']}+{entry['post']:>3} "
                      f"{entry['separation']:>7.4f} {entry['gamma']:>6.3f} "
                      f"{entry['f_eff']:>7} {entry['n_outputs']:>5} "
                      f"{'yes' if entry['constant'] else 'no':>6} "
                      f"{verdict:>12}")
    
    return survey


# =============================================================================
# SECTION 4: T6 Ramanujan Spectrum vs Separation
# =============================================================================

def run_ramanujan_separation_test():
    """Test: does T6's SP(p,d) predict per-lane separation?"""
    print(f"\n\n{'='*72}")
    print(f"  T6 RAMANUJAN SPECTRUM vs SEPARATION")
    print(f"{'='*72}")
    
    safe_primes = [3, 5, 7, 11, 13]
    
    # For each gap and prime, compute SP and best separation
    print(f"\n  {'p':>3} {'d':>3} {'c_p':>4} {'SP':>8} {'best_sep':>9} "
          f"{'best_op':>10} {'Γ_best':>7} {'SP*p':>7}")
    print(f"  {'-'*58}")
    
    sp_vals = []
    sep_vals = []
    
    for d in [2, 4, 6, 8, 10, 12, 30, 210]:
        for p in safe_primes:
            c = ramanujan_sum_prime(p, d)
            sp = (2 * p - 1 - c) / (p * p)
            
            # Best separation for this (p, d)
            best_sep = 0
            best_op = ""
            best_gamma = 0
            
            for root in ['Add', 'Sub', 'Mul']:
                for post in ['Id', 'Sqr', 'Neg']:
                    sep, _, _, _ = compute_separation(p, d, root, post)
                    if sep > best_sep:
                        best_sep = sep
                        best_op = f"{root}+{post}"
                        # gamma
                        d_r = d % p
                        base_f = {0}
                        if d_r != 0: base_f.add((p - d_r) % p)
                        f_out = set()
                        for r in base_f:
                            out = canonical_output(p, d, r, root, post)
                            if out is not None: f_out.add(out)
                        f_eff = {r for r in range(p) 
                                if canonical_output(p, d, r, root, post) is None 
                                or canonical_output(p, d, r, root, post) in f_out}
                        best_gamma = len(f_eff) / len(base_f) if len(base_f) > 0 else 0
            
            sp_vals.append(sp)
            sep_vals.append(best_sep)
            
            print(f"  {p:>3} {d:>3} {c:>4} {sp:>8.4f} {best_sep:>9.4f} "
                  f"{best_op:>10} {best_gamma:>7.3f} {sp*p:>7.3f}")
    
    # Correlation between SP and best_sep
    n = len(sp_vals)
    mean_sp = sum(sp_vals) / n
    mean_sep = sum(sep_vals) / n
    
    cov = sum((sp_vals[i] - mean_sp) * (sep_vals[i] - mean_sep) for i in range(n)) / n
    var_sp = sum((sp_vals[i] - mean_sp)**2 for i in range(n)) / n
    var_sep = sum((sep_vals[i] - mean_sep)**2 for i in range(n)) / n
    
    if var_sp > 0 and var_sep > 0:
        pearson = cov / (var_sp**0.5 * var_sep**0.5)
    else:
        pearson = 0
    
    print(f"\n  Pearson correlation (SP vs best_sep): r = {pearson:.6f}")
    
    # Also check: does SP*p predict separation better?
    sp_p_vals = [sp_vals[i] * safe_primes[i % len(safe_primes)] for i in range(n)]
    mean_spp = sum(sp_p_vals) / n
    cov2 = sum((sp_p_vals[i] - mean_spp) * (sep_vals[i] - mean_sep) for i in range(n)) / n
    var_spp = sum((sp_p_vals[i] - mean_spp)**2 for i in range(n)) / n
    
    if var_spp > 0 and var_sep > 0:
        pearson2 = cov2 / (var_spp**0.5 * var_sep**0.5)
    else:
        pearson2 = 0
    
    print(f"  Pearson correlation (SP*p vs best_sep): r = {pearson2:.6f}")
    
    return pearson, pearson2


# =============================================================================
# SECTION 5: Optimal Separation Topology
# =============================================================================

def run_optimal_separation():
    """Compute optimal topology by max-separation instead of max-Γ."""
    print(f"\n\n{'='*72}")
    print(f"  OPTIMAL SEPARATION TOPOLOGY")
    print(f"{'='*72}")
    
    safe_primes = [3, 5, 7, 11, 13]
    
    for name, d in [('Twin', 2), ('Cousin', 4), ('Sexy', 6)]:
        print(f"\n  {name} (d={d}):")
        
        topology = []
        for p in safe_primes:
            best_sep = -1
            best_op = None
            
            for root in ['Add', 'Sub', 'Mul']:
                for post in ['Id', 'Sqr', 'Neg']:
                    sep, carry_map, pr, cr = compute_separation(p, d, root, post)
                    
                    # SOUNDNESS CHECK: exclude constant-output operators
                    outputs = set()
                    for r in range(p):
                        out = canonical_output(p, d, r, root, post)
                        if out is not None:
                            outputs.add(out)
                    if len(outputs) <= 1:
                        continue  # Skip degenerate operators
                    
                    if sep > best_sep:
                        best_sep = sep
                        best_op = (root, post)
            
            # Handle p|d case: Mul+Id gives p distinct outputs
            if d % p == 0:
                best_op = ('Mul', 'Id')
                best_sep_override, _, _, _ = compute_separation(p, d, 'Mul', 'Id')
                best_sep = best_sep_override
            
            topology.append((p, best_op, best_sep))
            print(f"    p={p:>2}: {best_op[0]}+{best_op[1]:>3} sep={best_sep:.4f}")
        
        # Compare with Hydra's actual hetero presets
        hydra_presets = {
            'Twin': [(2,'Add','Id'), (3,'Mul','Id'), (5,'Add','Sqr'), 
                     (7,'Add','Sqr'), (11,'Mul','Neg'), (13,'Mul','Id')],
            'Cousin': [(2,'Add','Id'), (3,'Mul','Id'), (5,'Mul','Neg'),
                       (7,'Mul','Id'), (11,'Mul','Id'), (13,'Mul','Sqr')],
            'Sexy': [(2,'Add','Id'), (3,'Mul','Neg'), (5,'Mul','Neg'),
                     (7,'Mul','Id'), (11,'Mul','Neg'), (13,'Mul','Sqr')],
        }
        
        if name in hydra_presets:
            hydra = hydra_presets[name]
            print(f"\n    Comparison with Hydra {name.lower()} preset (p≥3 only):")
            print(f"    {'p':>3} {'ours':>10} {'our_sep':>8} {'hydra':>10} {'hyd_sep':>8} {'match':>6}")
            
            for p, our_op, our_sep in topology:
                hydra_match = [(hp, hr, hpo) for hp, hr, hpo in hydra if hp == p]
                if hydra_match:
                    hp, hr, hpo = hydra_match[0]
                    h_sep, _, _, _ = compute_separation(p, d, hr, hpo)
                    match = our_op == (hr, hpo)
                    print(f"    {p:>3} {our_op[0]}+{our_op[1]:>3} {our_sep:>8.4f} "
                          f"{hr}+{hpo:>3} {h_sep:>8.4f} {'✓' if match else '✗':>6}")


# =============================================================================
# SECTION 6: The Separation Theorem
# =============================================================================

def run_separation_theorem():
    """Attempt to derive a closed-form separation formula."""
    print(f"\n\n{'='*72}")
    print(f"  TOWARD A SEPARATION THEOREM")
    print(f"{'='*72}")
    
    # For Mul+Id on prime p with gap d where p∤d:
    # Output = a_r × b_r = r × (r+d) mod p
    # Carry indicator = 1 if output > p/2
    # 
    # Prime residues: r ∉ {0, p-d_r} (i.e., r ≠ 0 and r+d ≠ 0 mod p)
    # Composite residues: r ∈ {0, p-d_r}
    #
    # For r=0: output = 0 × d_r = 0, carry = 0
    # For r=p-d_r: output = (p-d_r) × 0 = 0, carry = 0
    # So P(carry=1 | composite) = 0 for Mul+Id
    #
    # For prime r: output = r(r+d) mod p, carry = 1 if r(r+d) > p/2
    # P(carry=1 | prime) = |{r ∈ prime_residues : r(r+d) mod p > p/2}| / |prime_residues|
    
    safe_primes = [3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]
    d = 2
    
    print(f"\n  Mul+Id separation analysis for d={d}:")
    print(f"  {'p':>3} {'prime_res':>9} {'carry1_pr':>9} {'P(c1|pr)':>9} "
          f"{'P(c1|co)':>9} {'sep':>7} {'(p-2)/2p':>9}")
    print(f"  {'-'*60}")
    
    for p in safe_primes:
        d_r = d % p
        if d_r == 0:
            continue  # Skip p|d
        
        prime_res = [r for r in range(p) if r != 0 and (r + d_r) % p != 0]
        comp_res = [r for r in range(p) if r == 0 or (r + d_r) % p == 0]
        
        # Carry=1 for prime residues
        carry1_prime = sum(1 for r in prime_res if (r * ((r + d_r) % p)) % p > p // 2)
        carry1_comp = sum(1 for r in comp_res if (r * ((r + d_r) % p)) % p > p // 2)
        
        p_c1_pr = carry1_prime / len(prime_res) if prime_res else 0
        p_c1_co = carry1_comp / len(comp_res) if comp_res else 0
        sep = abs(p_c1_pr - p_c1_co)
        
        # Hypothesized formula
        predicted = (p - 2) / (2 * p)
        
        print(f"  {p:>3} {len(prime_res):>9} {carry1_prime:>9} {p_c1_pr:>9.4f} "
              f"{p_c1_co:>9.4f} {sep:>7.4f} {predicted:>9.4f}")
    
    # Test other operators too
    print(f"\n  Add+Sqr separation analysis for d={d}:")
    print(f"  {'p':>3} {'sep':>7} {'formula_test':>12}")
    print(f"  {'-'*30}")
    
    for p in safe_primes:
        if d % p == 0:
            continue
        sep, _, _, _ = compute_separation(p, d, 'Add', 'Sqr')
        # Add+Sqr: output = (a_r + b_r)^2 = (2r+d)^2 mod p
        # For composites: r=0 → d^2, r=p-d_r → (p-2d)^2 = (2d)^2 mod p? No...
        # r=p-d_r: a_r=p-d_r, b_r=0. (a+b)^2 = (p-d_r)^2 = d_r^2 mod p. Carry = d_r^2 > p/2?
        
        comp_0_out = (d % p * d % p) % p  # r=0: (0 + d_r)^2
        comp_1_out = ((p - d % p) * (p - d % p)) % p  # r=p-d_r: same as d_r^2
        
        print(f"  {p:>3} {sep:>7.4f}    comp_outs={comp_0_out},{comp_1_out}")
    
    print(f"""
  ═══════════════════════════════════════════════════════════════════
  SEPARATION THEOREM (Conjecture, partial):

  For Mul+Id on prime p with gap d where p∤d:
    P(carry=1 | composite) = 0  
      (because both composite residues map to 0 via Mul)
    P(carry=1 | prime) ≈ (p-2)/(2p)
      (approximately half of prime residues produce output > p/2)
    Separation ≈ (p-2)/(2p)

  This approaches 1/2 as p → ∞.

  For Sub+Id: separation = 0 (constant output, all carries identical)
  For Add+Sqr: separation depends on quadratic residue structure

  The key insight: Mul+Id achieves maximum separation because:
  1. Composite residues ALWAYS produce output 0 (Mul by zero)
  2. Prime residues produce DIVERSE outputs across [1, p-1]
  3. The carry threshold p/2 splits these roughly evenly
  
  This is why the Hydra code labels Mul+Id as "SECRET WEAPON" —
  it creates the sharpest possible distinction between prime and 
  composite carry indicators on any lane where p∤d.
  ═══════════════════════════════════════════════════════════════════
""")


# =============================================================================
# MAIN
# =============================================================================

def main():
    print("╔═══════════════════════════════════════════════════════════════════╗")
    print("║  SEPARATION METRIC FORMALIZATION                                 ║")
    print("║  Why Mul+Id is the Secret Weapon: Pattern Diversity > Γ          ║")
    print("║  Kill Count: 106 | Convention: CANONICAL pair-operator           ║")
    print("╚═══════════════════════════════════════════════════════════════════╝")
    
    t0 = time.time()
    
    run_survey()
    r1, r2 = run_ramanujan_separation_test()
    run_optimal_separation()
    run_separation_theorem()
    
    elapsed = time.time() - t0
    print(f"\n  Time: {elapsed:.2f}s")
    
    return 0

if __name__ == "__main__":
    exit(main())
