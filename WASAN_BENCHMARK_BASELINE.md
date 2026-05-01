# Wasan Benchmark Baseline

Date: 2026-05-01

Command used:

```powershell
cd C:\Users\hackf\Projects\ns3d-n32-ntt6\arena
cargo run -p wassan_engine --bin wasan_bench --release -- --samples 8 --bank-size 16
```

Configuration:

- samples: 8
- bank size: 16
- CRT lanes used by benchmark context: 4 verified primes
- hologram capacity: 432 bytes

## First-pass results

| Case | Iterations | Total ms | Avg us |
|------|-----------:|---------:|-------:|
| encode_small | 8 | 1.187 | 148.325 |
| encode_half | 8 | 1.172 | 146.512 |
| encode_full | 8 | 1.071 | 133.825 |
| decode_full | 8 | 0.006 | 0.775 |
| decode_verified_full | 8 | 1.092 | 136.512 |
| syndrome_clean | 8 | 1.081 | 135.088 |
| self_correct_3_sectors | 8 | 2.498 | 312.275 |
| recover_from_sectors | 8 | 3.300 | 412.512 |
| engine_store_batch | 1 | 4.001 | 4001.300 |
| engine_retrieve_by_id | 8 | 0.002 | 0.300 |
| engine_retrieve_by_content | 8 | 1.547 | 193.425 |

## What this baseline says

- encode, verified decode, and clean syndrome checks are all in the same rough
  cost band on this first run
- direct decode and retrieve-by-id are effectively negligible compared to the
  NTT-heavy operations
- sector recovery and self-correction are more expensive than clean-path
  inspection, as expected
- content retrieval cost reflects the attractor-bank scan path, not just a
  direct key lookup

## Next benchmark passes worth collecting

1. rerun with `--samples 64` and `--bank-size 64`
2. rerun with `--samples 256` after a warm cache pass
3. compare 4-lane vs extended-lane contexts if the code path is widened
4. add NS persistence-path timing for `ns_step_wassan()` and `safe_snapshot()`
5. capture cold-build time separately from steady-state runtime

## Notes

- This is a first baseline, not a publication-grade benchmark campaign.
- The benchmark binary lives at
  `arena/crates/wassan_engine/src/bin/wasan_bench.rs`.
