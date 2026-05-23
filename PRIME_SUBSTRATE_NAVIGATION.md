# Prime Substrate Navigation

This note records what the code calls "sieves" as CRAM/Hydra navigation heads. They are not classical prime-only filters. They read and organize the prime, coprime, and composite substrate by residue operators, carry signatures, winding identities, and family coordinates.

## Head Anatomy

Hydra heads are built from toric lanes. Each lane has a modulus, a root operator such as `Add`, `Sub`, or `Mul`, and a post operator such as `Id`, `Neg`, or `Sqr`. A head calibrates against known structure, records carry/signature patterns, and uses those patterns to classify candidates.

Important local paths:

- `arena/crates/prime_hunt/src/hydra.rs` - single Hydra heads and presets.
- `arena/crates/prime_hunt/src/multi_hydra.rs` - multi-head lattice maps and signatures.
- `arena/qmnf_unified/python/hydra/hydra_sieve_v3.py` - Python head runner.
- `arena/qmnf_unified/python/hydra/hydra_optimal_ops.py` - operator search across primes/families.

## What The Heads Do

The heads expose structure. They can eliminate impossible candidates, but that is only one readout. The same machinery maps prime families, tracks coprime/composite carriers, locates doppelganger classes that look identical under weaker heads, and supplies residue coordinates for other systems such as NS fluid reads and Grover-on-classical torus navigation.

MultiHydra is the clearest form: each head is a different CRAM lens on the coprime lattice. Combining heads gives a higher-dimensional fingerprint for family structure, density strata, and carrier behavior.

## Current Run Notes

Commands run on 2026-04-30:

- `cargo test -p prime_hunt -- --nocapture` passed 103 tests.
- `python arena/qmnf_unified/python/hydra/hydra_sieve_v3.py` passed 26/26 wiring checks, exact K-Elimination for 10,000 values, and zero false negatives for 205 twins in `[3,10000)`.
- `python arena/qmnf_unified/python/hydra/hydra_optimal_ops.py` found the best tested single twin head as Champion S6 Hetero `[2,3,5,7,11,13]` at 79.80% with zero false negatives.

At `N=200000`, the Python Hydra runner reported twin 79.8%, cousin 75.2%, sexy 56.2%, `d=30` 56.6%, and `d=360` 45.5%, all with zero false negatives. These numbers should be treated as observed navigation/exposure rates for the tested operators and bounds, not as final ontology claims.

## Safe-Base Discipline

Stable work stays anchored in the safe base through 19. Larger capacity should be built through composites and prime powers, phase-locked through 11 when required. Useful carriers include `510510 = 2*3*5*7*11*13*17`; larger probes must be audited as probes unless wrapped into the safe-base composite/prime-power layer.
