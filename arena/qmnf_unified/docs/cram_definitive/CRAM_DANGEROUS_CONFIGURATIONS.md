# CRAM DANGEROUS CONFIGURATIONS, FAILURE MODES, AND ATTACK SURFACES

**"It is ethical incompetence if we don't explore and understand these areas most of all."**

31 dangers identified. 6 CRITICAL. 13 OPEN (no defense known). 12 categories.

## CRITICAL (6)

| ID | Title | Category | Status |
|---|---|---|---|
| D-004 | Non-coprime moduli destroy CRT uniqueness | COPRIME | MITIGATED by construction |
| D-005 | Exact prime from {2,3,5,7,11} family | COPRIME | MITIGATED by prime selection |
| D-012 | Garner j-loop must go ASCENDING | GARNER | MITIGATED — fixed Mar 29 |
| D-014 | Python / produces float on int inputs | FLOAT | MITIGATED by schema design |
| D-021 | Clockwork Bootstrap depends on K-Elim being EXACT | CRYPTO | PARTIAL — Lean proof exists |
| D-024 | Discrete regularity ≠ continuum regularity | NS | OPEN — this IS the Millennium Problem |

## HIGH (12)

| ID | Title | Category | Status |
|---|---|---|---|
| D-001 | Basis including prime 2 with Mul | DKAM | MITIGATED by architectural rule |
| D-006 | Div with non-invertible b → silent pass-through | DIVISION | OPEN |
| D-007 | Inv on composite with non-invertible element | DIVISION | OPEN |
| D-009 | Sqr carry pattern leaks QR of input | SIDECHANNEL | OPEN for Python |
| D-011 | Inv/Div timing variation | SIDECHANNEL | OPEN for test code |
| D-015 | CF terms beyond ~N/1.03 are artifacts | FLOAT | MITIGATED by trim protocol |
| D-016 | N additions accumulate N×ε | ERROR | MITIGATED by tracking |
| D-018 | Band overwrite destroys data | WASSAN | OPEN |
| D-022 | AHOP depth < 64 → security < 2¹²⁸ | CRYPTO | MITIGATED by parameter |
| D-025 | Re > 2000 may overflow i128 stencils | NS | PARTIAL |
| D-026 | ω^crt ≠ ω^phys (OP-1) | NS | OPEN |
| D-030 | CRT residues are correlations, not causes | ASTRO | MITIGATED by document structure |

## OPEN ATTACK SURFACE (13 items — the work that remains)

1. **D-002**: No automatic degree tracking through computation graphs
2. **D-006**: Div silent pass-through hides division failure
3. **D-007**: Inv on composite moduli needs nilpotent-aware handling
4. **D-009**: Carry pattern side channel (need constant-time implementation)
5. **D-010**: Schema choice reveals computation type (no obfuscation)
6. **D-011**: Timing variation in Inv/Div (need constant-time modular inverse)
7. **D-017**: Mul by large value can blow error budget (no auto re-quantization)
8. **D-018**: Band overwrite is destructive (no versioning)
9. **D-019**: No fold depth cap (can fold to meaningless depths)
10. **D-023**: Shadow Entropy quality at high throughput unvalidated
11. **D-024**: Discrete ≠ continuum (Millennium Problem)
12. **D-026**: CRT↔physical norm compatibility (OP-1)
13. **D-031**: Population study needed for planetary schema significance

## HONESTLY KILLED (mitigated, but document WHY)

- D-004/D-005: Coprimality enforced by construction (exact primes > 11, safe basis ≤ 11)
- D-012: Garner loop bug FOUND and FIXED (regression test in place)
- D-014: Float contamination prevented by schema (/ never called)
- D-001: Prime 2 excluded from working basis (parking only)

## THE PRINCIPLE

Every system has an attack surface. Refusing to document it doesn't shrink the surface — it just means the attacker sees it before you do. This document exists so the defender always has at least as much information as the attacker.

The Atlas shows what works. This document shows what breaks. Both are necessary.
