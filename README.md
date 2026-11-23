# NS 3D Solver N32

A Rust research workspace for integer Navier–Stokes experiments on a periodic \(32^3\) grid and the modular arithmetic that supports them. It includes canonical integer and CRT v4 solver paths, a separate six-prime number-theoretic transform, the CRAM and sieve workspaces, and Wasan storage.

## Build and verify

Use stable Rust and Cargo. From the repository root, run the focused release checks under `arena/`. The larger thread stack accommodates tests that allocate full \(32^3\) fields.

```sh
cd arena
RUST_MIN_STACK=67108864 cargo test --locked --release -p ns_regularity --lib ns3d_n32_crt_v4
RUST_MIN_STACK=67108864 cargo test --locked --release -p ns_regularity --lib ns3d_n32_ntt6
```

On PowerShell, set `$env:RUST_MIN_STACK = '67108864'` before running the two Cargo commands. For a workspace inventory, see [WORKSPACE_MANIFEST.md](WORKSPACE_MANIFEST.md).

## Solver paths and scope

| Path | Role |
|---|---|
| [Canonical integer step](arena/crates/ns_v5/src/ns3d_n32.rs) | Direct signed integer reference for the fixed N32 solver. |
| [CRT v4 step](arena/crates/ns_v5/src/ns3d_n32_crt_v4.rs) | Production residue path with four transform primes and two additional arithmetic lanes. Focused tests compare it with the canonical step. |
| [Six-prime NTT](arena/crates/ns_v5/src/ns3d_n32_ntt6.rs) | Independent spectral representation with its own unit tests. |
| [Signed shear comparison](https://github.com/Skyelabz210/NS-3D-solver_n32-ntt6/pull/1) | Draft regression that measures both production paths against the exact rational diffusion stencil for one signed shear. |

The solver uses truncation toward zero at signed division boundaries. The companion Lean split-step model uses Euclidean floor for positive denominators. Their finite updates can diverge on the same input. Their relation to the continuum remains an analytical question. [Open seams](arena/crates/ns_v5/docs/OPEN_SEAMS.md) · [Companion NS-Regularity repository](https://github.com/Skyelabz210/NS-Regularity)

## Related workspace components

- [arena/crates/](arena/crates/) contains the NS solver, CRT arithmetic, Wasan storage, and runtime and bridge crates.
- [arena/qmnf_unified/](arena/qmnf_unified/) contains the companion sieve, CRAM, and formalization trees.
- [imports/](imports/) contains incoming source snapshots.

Wasan preserves the solver’s residue lanes and tier structure across storage operations. The [Wasan engine](arena/crates/wassan_engine/) includes the `wasan_bench` release binary.

## License

Anthony Diaz’s original project code and documentation are available under the [MIT License](LICENSE). [Scope and upstream notices](THIRD_PARTY.md)
