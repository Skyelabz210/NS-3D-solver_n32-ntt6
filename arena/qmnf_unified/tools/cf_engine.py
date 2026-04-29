#!/usr/bin/env python3
"""CF Engine for log₂(3) — CRAM Euclidean K-Elimination Chain
   
   mpmath binary splitting → exact P/Q → Euclidean = K-Elimination
   48,543 terms in 2.5s at D=50,000. Scales linearly.
   
   Zero float in the CF extraction. mpmath uses binary splitting
   (exact integer at core) for the logarithm computation.
"""
import sys, time, math, argparse
sys.set_int_max_str_digits(100_000_000)
import mpmath

def compute_cf(D, outfile=None):
    """Compute CF terms of log₂(3) at D-digit precision."""
    mpmath.mp.dps = D + 100
    
    t0 = time.time()
    print(f"  Computing log₂(3) to {D} digits...", end=" ", flush=True)
    log2_3 = mpmath.log(3) / mpmath.log(2)
    t1 = time.time()
    print(f"done ({t1-t0:.1f}s)")
    
    # Extract exact integer: P = floor(log₂(3) × 10^D)
    N = mpmath.mpf(10) ** D
    p_int = int(mpmath.floor(log2_3 * N))
    q_int = 10 ** D
    
    # Euclidean = K-Elimination chain
    print(f"  Euclidean K-Elimination...", end=" ", flush=True)
    t2 = time.time()
    a, b = p_int, q_int
    terms = []
    while b > 0:
        q, r = divmod(a, b)
        terms.append(int(q))
        a, b = b, r
    t3 = time.time()
    print(f"{len(terms)} raw terms ({t3-t2:.2f}s)")
    
    # Reliable depth
    reliable = int(D / 1.03)
    rterms = terms[:reliable]
    
    # OEIS check
    known = [1,1,1,2,2,3,1,5,2,23,2,2,1,1,55,1,4,3,1,1,15]
    oeis_ok = all(i < len(rterms) and rterms[i] == known[i] for i in range(len(known)))
    
    # Cross-checks
    checks = [(4312, 8228), (2764, 4878), (528, 3308), (330, 2436)]
    
    rmax = max(rterms) if rterms else 0
    rpos = rterms.index(rmax) if rterms else 0
    geo = math.exp(sum(math.log(max(a,1)) for a in rterms) / len(rterms))
    
    print(f"\n  Reliable CF terms: {len(rterms):,}")
    print(f"  OEIS A028507: {'21/21 ✓' if oeis_ok else 'FAIL'}")
    for k, exp in checks:
        if len(rterms) > k:
            ok = rterms[k] == exp
            print(f"  a_{k}: {rterms[k]} (expected {exp}) {'✓' if ok else '✗'}")
    print(f"  Max: a_{rpos} = {rmax:,}")
    print(f"  Geometric mean: {geo:.4f} (Khinchin = 2.6854)")
    print(f"  Required for Collatz: 2,147,483,648")
    print(f"  Gap: {2**31 // max(1, rmax):,}×")
    print(f"  Total time: {time.time()-t0:.1f}s")
    
    # Top 15
    top = sorted(enumerate(rterms), key=lambda x: -x[1])[:15]
    print(f"\n  TOP 15:")
    for r, (k, a) in enumerate(top, 1):
        print(f"    #{r:>2}: a_{k:<6} = {a:>8,}")
    
    # Save
    if outfile:
        with open(outfile, 'w') as f:
            f.write(f"# CF(log2(3)) D={D} reliable={len(rterms)} max={rmax}\n")
            for k, a in enumerate(rterms):
                f.write(f"{k},{a}\n")
        print(f"\n  Saved {len(rterms)} terms to {outfile}")
    
    return rterms

if __name__ == "__main__":
    p = argparse.ArgumentParser(description="CF of log₂(3) via CRAM K-Elimination")
    p.add_argument("-d", "--digits", type=int, default=50000, help="Precision in digits")
    p.add_argument("-o", "--output", type=str, default="cf_terms.csv", help="Output file")
    args = p.parse_args()
    compute_cf(args.digits, args.output)
