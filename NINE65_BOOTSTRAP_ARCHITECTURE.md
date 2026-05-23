# NINE65 Bootstrap Architecture Notes

NINE65 v7 is dual-mode FHE, not a single bootstrap story.

## Symmetric / Key-Holder Mode

The key-holder path avoids public bootstrapping. When the evaluator has access to the secret key, the refresh operation is protected decrypt-and-reencrypt:

1. decrypt the current ciphertext,
2. fresh-encrypt the recovered plaintext,
3. zeroize the temporary plaintext,
4. wrap the exposure window in the Three-Lock protection model.

Local source: `C:\Users\hackf\Projects\NINE65_v7\crates\nine65\src\ops\symmetric_bootstrap.rs`.

This mode has no BSK, no KSK, no homomorphic decryption circuit, no boot-to-work modswitch, and no public bootstrap overhead. It is the path behind the bootstrap-free/key-holder framing.

## Public / Evaluator Mode

The public path uses Clockwork Bootstrap and `AutoBootstrapEvaluator`.

Clockwork has three phases:

1. `ModSwitch Q -> t`;
2. homomorphic inner product using bootstrap key material;
3. `ModSwitch Q_boot -> Q_work`, dropping one boot prime and recomputing anchor limbs.

Local sources:

- `C:\Users\hackf\Projects\NINE65_v7\crates\nine65\src\ops\bootstrap.rs`
- `C:\Users\hackf\Projects\NINE65_v7\crates\nine65\src\ops\auto_bootstrap.rs`
- `C:\Users\hackf\Projects\NINE65_v7\docs\BOOTSTRAP_CORRECTNESS_CONTRACT.md`

The important public-mode fix in commit `50f221f` was in `AutoBootstrapEvaluator::mul_auto()`: budget exhaustion must force a bootstrap. Previously, `consume()` could return `Err` while leaving the budget above the trigger threshold, so the automatic refresh did not fire reliably.

## Correctness Invariants

Clockwork is guarded by explicit invariants:

- boot primes strictly contain the work primes;
- exactly one extra boot prime is dropped;
- boot anchors match canonical anchor primes;
- anchor limbs are recomputed after prime drop, not copied or zeroed;
- non-circular KSK mode key-switches before modswitch;
- CRT reconstruction must reject overflow instead of wrapping.

These invariants are the real solution to the bootstrap fragility: the bootstrap is not treated as a black box. The code pins every modulus-space transition.

## Current Evidence In This Checkout

The local history records `50f221f` as:

- symmetric depth ceiling investigation;
- public auto-bootstrap depth validation;
- bootstrap accumulation bias testing;
- auto-bootstrap trigger fix.

Its commit message reports public mode at 100 chained multiplications with all checkpoints correct, 99 auto-bootstraps, and zero drift. It also records no directional bias over repeated bootstrap cycles.

There is mixed local wording around symmetric depth. Some docs describe depth-50+ or depth-200 targets, while the `50f221f` commit message reports raw repeated symmetric ct×ct squaring as noise-limited much earlier for the tested secure configs. Treat symmetric depth claims as workload-specific until the exact run matrix is attached.
