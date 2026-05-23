# QWEN.md - NS 3D N32 NTT6 Project

## Project Overview

This repository contains two related but distinct systems with shared genealogy:
1. **Mother Prime Sieve** - A discovery tool for mathematical structures
2. **NS 3D N32 NTT6 Solver** - A Navier-Stokes solver using exact-integer arithmetic and 6-prime NTT spectral representation

The project has reached the P5B-9A gate-pass state as of 2026-04-29, with 58/58 tests passing. The main achievement is a 6-prime spectral computation that widens the Biot-Savart wall from M_NTT_4/2 ≈ 5×10^11 to M_NTT_6/2 ≈ 1.13×10^18, representing a 2.25 million× improvement in computational limits.

## Architecture

### Two-Mechanism Terminology
- **4p spectral witness basis**: {769, 929, 1153, 1217} - Used for Garner4 reconstruction of raw_BS inside biot_savart_residue_native
- **6p division/state basis**: {769,929,1153,1217,1409,1601} + {2^31, 5^9} - Used for K-Elim division quotients and ExtRes8 state representation

### Key Constraints
- **Hard precondition**: If raw_BS exceeds M_NTT_4 / 2 ≈ 5.01×10¹¹, Garner4 aliases
- The 6p basis does NOT remove this precondition — it inherits it

## Building and Running

### Prerequisites
- Rust toolchain (Cargo)

### Build Commands
```bash
cd arena
cargo build --workspace
cargo test --workspace
```

### Specific Gate Test
```bash
cargo test -p d024_probe per_prime_spectral_residues_crt_consistent
```

## Key Features

### Exact Integer Arithmetic
- No floating-point operations in computation paths
- No `f32`, `f64`, `as f64`, `floor()`, `round()` in computation paths
- Display-only conversions are tolerated

### 6-Prime NTT Extension
- Added primes p5=1409 and p6=1601 to extend the basis
- New corridor limit: M_NTT_6/2 ≈ 1.13×10^18 (2.25 million× larger than 4-prime limit)
- Maintains compatibility with existing 4-prime computations

### Core Crates and Their Functions

#### Foundational Crates
- `qmnf_primitives` - Single source of truth for modular arithmetic, NTT, K-Elimination, Garner reconstruction, and Residue types
- `qmnf_crtbigint` - Extended CRT BigInt implementation with seven-layer exact computation architecture:
  - Layer 0: Modular primitives (mulmod, gcd, mod_inverse)
  - Layer 1: Exact CRT Lanes: 8×63-bit primes (~504-bit capacity)
  - Layer 2: Safe Basis Composites: {2,3,5,7,11,13} phase-locked sub-lanes
  - Layer 3: Safe Anchor: single large prime for O(1) magnitude recovery
  - Layer 4: WASSAN Bands: 144 φ-harmonic execution lanes
  - Layer 5: PLMG Rails/Voids: 28% valid / 72% free error detection
  - Layer 6: Double Helix: dual-strand compute + verify
  - Features exact rational numbers, integer sine approximations, and safe anchor mechanisms

#### Core NS Solver Crates
- `nsgrid3d` - 3D Navier-Stokes on exact integer substrate with discrete Biot-Savart
  - Contains biot_savart, fused_solver, regularity_tracker, and other core NS operators
- `ns_v5` (ns_regularity) - Exact-Integer Navier-Stokes Regularity via CRT Contraction
  - Contains the core arithmetic and exact-rational checks
  - Implements the Recumbent overflow geometry and WASSAN structural persistence
  - Houses the 3D exact solver with exact 3D operator family
  - Includes ns3d_n32, ns3d_n32_crt, cram_div, and ns3d_n32_ntt6 modules
- `ns_winding_v2` - 2D NS exact-energy + lifted winding

#### Advanced Algorithm Crates
- `d024_probe` - Diagnostic and probe tools for NS analysis
  - Includes attractor tests, K_b symmetry tests, b-component attractor tests
  - Contains corridor probe tools (corridor_probe_v3) for measuring computational limits
- `cram_div` - Residue-pure exact integer division in extended CRT basis
  - Implements CRAM (Compressed Representation and Arithmetic Method) for exact division
  - Extends CRT basis with small-prime-power lanes (2^31, 5^9) to absorb divisors
  - Provides exact division for divisors that are products of {2, 5} only
  - Includes DivCtx for pre-computed per-divisor inverses to eliminate repeated modinv calls
  - Supports both truncating division (trunc_div_signed) and exact division (exact_div_signed_zero_remainder)
- `cram_explorer` - Tools for exploring mathematical structures (Apollonian, holodrive, k_elim, projector, shadow_ladder, torus)
- `qcid_ns_bridge` - Fp² dual-track NS bridge for connecting different computational domains
  - Implements Fp2 elements (a + b√δ) with physics on residue (a) and continuum info on amplitude (b)
  - Provides dual-track state management with independent physics and contraction tracking
  - Includes norm computation (a² - δb²) as the bridge quantity between domains
  - Features Frobenius conjugate (σ(a+b√δ) = a-b√δ) for amplitude flipping while preserving residue
  - Contains ring homomorphism properties for integer embedding in Fp2
  - Supports nested radical computations for N=16 and N=32 FFTs

#### Holographic Storage and Processing
- `hdv_crt` - Hyperdimensional CRT vectors
- `wassan_holographic` - 144-band NTT holographic storage
  - Implements holographic data storage with 144 data bands and 112 parity bands
  - Uses NTT transforms to create holographic sectors where each sector contains info about all coefficients
  - Supports error correction with up to 56 band errors (112 parity bands total)
  - Features phase-locked retrieval via coherence matching
  - Provides self-correction via attractor basin convergence
  - Zero-float architecture with exact modular integer arithmetic
- `wassan_engine` - WASSAN band arithmetic and memory management
  - Implements convergence tracking with syndrome detection and correction
  - Manages attractor banks for phase-locked memory retrieval
  - Tracks stability scores and correction statistics
  - Provides basin creation, storage and retrieval mechanisms
- `plmg_validator` - Rail/void phase-lock validation

#### Search and Optimization
- `mmbf_grover_bridge` - Ramanujan-Grover formula implementation for optimization
  - Implements the Ramanujan-Hydra Spectral Correspondence (T6) as an oracle
  - Provides Dense Toric Grover search engine for prime sieving
  - Uses K-Elimination for exact reconstruction
  - Features the Ramanujan-Grover formula: T_opt = ⌊π/4 · p / √(2p - 1 - c_p(d))⌋
  - Supports two-tier oracle: classical (p ∈ {2,5,13}) and Grover-admissible (p ∈ {3,7,11})
  - Provides 141× speedup for large primes (p~10^4) vs classical enumeration
  - Includes sieve power computation and forbidden residue classification
- `qedde_v2` - Cell, CompositeSchema, HydraEngine, and PostHoc schemas
  - Implements interchangeable schema cell architecture with hot-swappable computation units
  - Features CompositeSchema for chaining multiple schemas in sequence
  - Includes PhaseGate for inter-schema handoff with integrity checks
  - Provides NS-specific schemas: turbulence, vortex, multi-scale, and regularity analysis
  - Contains Kähler/Picard-Fuchs schemas for geometric invariant computation
  - Offers schema factory for building complete pipelines (NS, Yukawa-Kähler, turbulence)
  - Implements OracleA/B for structural exclusion and objective scoring
  - Supports diagnostic sinks for winding/entropy/residual tracking
- `qedde_v3` - Unified substrate architecture integrating QEDDE, HackFate, and MMBF
  - Features WASSAN drive as shared storage substrate for all operations
  - Implements Double Helix verifier for every write/read operation
  - Provides Attractor Memory cells with self-correction capabilities
  - Includes resident oracles (Grover, Ramanujan, Shor) available to all cells
  - Features CRT Beacon as shared accelerator for K-Elimination and reconstruction
  - Implements Shadow Entropy pool for tamper-resistant operation
  - Supports multiple cell types: PrimeSieve, NSVelocity, NSVortex, NeuralForward, etc.
  - Provides fold function for irreversible destruction when tamper detected
  - Uses Fibonacci-scheduled verification phases
  - Implements Descartes phase-lock verification across four sectors
  - Features tamper detection and automatic fold triggering
  - Provides entropy harvesting from computation byproducts
- `hydra_v3` - Hydra sieve algorithm for prime elimination
  - Implements multi-dimensional operator-toric carry-propagation sieve
  - Features heterogeneous optimal operators per prime and family
  - Includes K-Elimination magnitude oracle for exact reconstruction
  - Provides PLMG rail/void validator for O(1) geometric checks
  - Supports multiple prime families: twin, cousin, sexy, Sophie Germain, Goldbach, Polignac
  - Implements carry spectral fingerprinting for family characterization
  - Offers 79.8% elimination rate for twin primes with 0 false negatives
  - Includes operator separation tables for per-prime optimization
  - Features calibrated oracle atlas with 99.9% recall and 5.22× enrichment

#### Security and Telemetry
- `loki_engine` - Threat detection
- `cdhs_telemetry` - Health diagnostics

#### Cognitive and Advanced Math
- `asve_engine` - Adaptive Self-Validating Engine (cognitive engine)
  - Implements cognitive reentry loop: PERCEIVE → REMEMBER → ANALYZE → HYPOTHESIZE → VALIDATE → LEARN → SECURE
  - Features integer-only cognitive state with quantized metrics (coherence, confidence, novelty, threat)
  - Maintains knowledge graph with concepts, relations and validation
  - Detects cognitive gaps and generates hypotheses to address them
  - Includes paradigm shift mechanism for axiom suspension and re-evaluation
  - Provides NLG hedge level based on coherence score
  - Supports WASSAN integration for memory storage and retrieval
- `time_crystal` - Temporal coherence structures
  - Implements phi-resonant integer oscillator with Fibonacci aperiodic gating
  - Provides temporal heartbeat for HackFate systems (Loki, CDHS, ASVE)
  - Uses golden ratio approximation as integer fraction (PHI_NUM/PHI_DEN)
  - Features Digital Time Crystal with oscillator and gate components
  - Generates aperiodic gate events at Fibonacci-indexed tick counts
  - Produces high-quality deterministic entropy for CT-DRBG
  - Includes phase readout for health monitoring
- `svd_engine` - SVD decomposition
  - Implements integer-only SVD engine with power iteration for dominant singular value
  - Features Jacobi rotations via integer CORDIC for full SVD of small matrices
  - Computes coherence score as ratio of dominant singular value to Frobenius norm
  - Provides cognitive interface for ASVE to measure knowledge graph coherence
  - Uses scaled integer arithmetic (2^20 scale factor) for precision
  - Includes HDV similarity matrix coherence computation
  - Calculates effective rank estimation (Frobenius² / σ₁²)
- `maa_dh` - Apollonian Double Helix (AHOP post-quantum cryptography)
  - Implements Mutual Apollonian Attestation with dual CRT strands
  - Uses Descartes Circle Theorem as invariant for verification: (a+b+c+d)² = 2(a²+b²+c²+d²)
  - Features dual-strand computation with checkpoint-based verification
  - Provides corruption detection via strand disagreement
  - Generates Apollonian tree structures for attestation
  - Includes integer square root computation for Descartes verification
  - Supports post-quantum cryptographic applications through Apollonian packings

#### Specialized Mathematics
- `k_elim_period` - K-Elimination period analysis
  - Implements BSGS (Baby-Step Giant-Step) algorithm for multiplicative order computation
  - Fixes the broken doubling strategy that only found periods that are powers of 2
  - Provides O(√N) time complexity for period finding instead of exponential
  - Features period verification to ensure minimality and correctness
  - Implements batch period finder for all CRT lanes simultaneously
  - Computes winding period of CRT torus orbit where k=0 (energy neutral)
  - Achieves 106-232× speedup over naive methods for large primes
  - Includes safe basis lane testing across all primes {2,3,5,7,11,13}
- `landau_siegel_chimera` - Landau-Siegel zeros research
  - Implements topological containment of Landau-Siegel zeros via corrected chimera lattice
  - Features 5-tuple chimera coordinates with zero-flags for divisor class discrimination
  - Provides ×4 closed cycle on (Z/pZ)× for L-function decomposition
  - Implements QR/QNR structural bias analysis with harmonic averaging
  - Features loop invariance: |Forward - Reverse| = 0 for all tested D
  - Includes Double Chimera: Z/11²Z × Z/7²Z = Z/5929Z for combined constraint
  - Provides K-Elimination winding digit computation on Z/p²Z
  - Implements BSGS (Baby-Step Giant-Step) for multiplicative order computation
  - Shows QR positions have 1.83× harmonic average heavier than QNR positions
- `quantum_continuum_torus` - Quantum-continuum mathematical structures
  - Implements Quantum Continuum Injected Discrete Torus (QCIDT) architecture
  - Features F_p² quantum amplitude representation (a + b√δ) with Frobenius automorphism
  - Provides discrete readout via norm: N(a+b√δ) = a²-δb² (Frobenius-invariant)
  - Includes winding correction engine for prime detection with arrow of time computation
  - Implements resolution ladder with inter-resolution contraction injection
  - Features rung-aware sieve with 15-lane configuration achieving 90%+ precision
  - Incorporates Maya calendar-inspired chimera engine with Tzolkin, Haab, and Round layers
  - Provides K-Elimination bridge between quantum and discrete layers
  - Includes Copernican frame with observer-dependent derotation
  - Implements shadow-gated injection mechanism tied to prime 11 resonance
- `ramanujan_memory_lane` - Ramanujan-based memory structures
  - Implements cross-domain memory bridge via Ramanujan harmonic residues
  - Features 144-band WASSAN substrate for winding vector storage
  - Provides three-domain interpretation: CRAM/CRT, Fluid (NS), and Quantum
  - Includes shadow anchor lane (prime 11) that remains at MultidimensionalZero
  - Implements Permanent Non-Resonance Theorem: shadow lane never resonates with NS coefficients
  - Features spectral fingerprint bridge using sieve power SP(p,d) = (2p-1-c_p(d))/p²
  - Uses Ramanujan sums c_q(n) = q-1 if q|n, else -1 for harmonic computation
  - Includes HarmonicLane with operators (Add/Sub) and roles (Active/ShadowAnchor)
  - Implements band indexing: 28 bands per prime distributed across 144 bands
- `prime_hunt` - Prime number research tools
  - Implements coprimality-first prime detection algorithms
  - Features shadow gap sieve for identifying primes by finding residues that escape shadow network coverage
  - Includes admissible sieve that walks only coprime residue classes
  - Provides K-Elimination winding analysis for composite detection
  - Offers cousin pair (gap-4) detection with spectral analysis
  - Includes displacement power analysis for tracking missing primes in periods
  - Implements K-Vort accumulator for winding number evolution patterns
  - Uses SAFE_BASIS {2,3,5,7,11,13} for residue-based classification

#### Runtime and Utilities
- `hackfate_runtime` - Full lifecycle runtime
- `sanity_crt` - CRT sanity checking tools

## Development Conventions

### Testing Strategy
- Comprehensive test coverage with 58+ passing tests
- CRT consistency validation across all computation paths
- Per-prime spectral residue validation
- Boundary condition testing

### Code Organization
- Code organized in a Rust workspace under `arena/`
- Multiple crates with specific responsibilities (see Cargo.toml)
- Clear separation between the Mother Prime Sieve and NS Solver systems
- Extensive documentation in markdown files

### Naming Convention
- Files and modules follow systematic naming patterns (e.g., `ns3d_n32_ntt6_P5B9A_prereqs.rs`)
- Versioned implementations (v1, v2, v3, etc.) to track evolution
- Descriptive names for functions and variables related to mathematical concepts

## Project Status

- **Current State**: P5B-9A gate: PASS
- **Test Status**: 58/58 tests passing
- **Next Steps**: Implement true 6-prime spectral Biot-Savart computation
- **License**: Private, not for redistribution

## Key Documentation Files

- `P5B_DAG_P5B9A_GATE.md` - Master DAG with architectural notes
  - Defines the corrected architecture with two-mechanism terminology
  - Details the 4p spectral witness basis {769, 929, 1153, 1217} for Garner4 reconstruction
  - Specifies the 6p division/state basis {769,929,1153,1217,1409,1601} + {2^31, 5^9}
  - Documents the hard precondition: raw_BS must not exceed M_NTT_4/2 ≈ 5.01×10^11
  - Includes the decisive P5B-9A gate result: per_prime_spectral_residues_crt_consistent: PASS ✓ (0 mismatches)
  - Outlines the ADDITIONAL GUARDS added: garner6_near_corridor_edge, p5_p6_seeding_from_garner4_forbidden, per_prime_spectral_residues_crt_consistent
  - Specifies the OPEN NODE: P5B-9A — True 6-Prime Spectral Biot-Savart implementation
  - Lists test counts (58/58) and defines the acceptance gates for the 6p BS implementation

- `PROPOSAL_P5B9A_EXECUTION.md` - Implementation plan for next phase
  - Details the implementation plan for true 6-prime spectral Biot-Savart computation
  - Outlines the step-by-step approach: Generalize ntt3d_lane, Implement biot_savart_6p_spectral, Add side-by-side validation test, Replace 4p spectral with 6p in ns_step_crt_v4
  - Specifies the forbidden shortcut: seeding p5/p6 from Garner4 result
  - Defines the acceptance gates: 58/58 existing tests still pass, per_prime_spectral_residues_crt_consistent: PASS, New 6p BS test passes, Zero float, zero i64 seam, all 5 division sites witnessed
  - Provides generalized NTT functions with variable spectral lane count
  - Includes side-by-side validation during implementation comparing 4p and 6p BS results

- `executioner_dag.md` - Execution DAG for various sessions
  - Documents the cognitive reentry loop architecture for ASVE engine: PERCEIVE → REMEMBER → ANALYZE → HYPOTHESIZE → VALIDATE → LEARN → SECURE
  - Details the integer-only cognitive state with quantized metrics (coherence, confidence, novelty, threat)
  - Specifies the architectural constants: N=32, NS_SCALE=10000, NTT_PRIMES=[769, 929, 1153, 1217]
  - Includes the DivContract table showing divisor analysis for N^6, S^2, 4S, 2S, N^2
  - Documents the corridor_probe_v3 findings: all 5 division contracts have huge headroom, BS raw corridor is the sole limit
  - Outlines the implementation roadmap with nodes P0A-P0D, P2A-P2B, P4A, P5A-P5C, and beyond
  - Details the checkpoint summary and pending items for future sessions

- `P5B_DAG_P5B9A_GATE.md` - Master DAG (corrected architecture)
  - Contains the two-mechanism terminology: 4p spectral witness basis and 6p division/state basis
  - Explains the critical correction: raw_p5 = Garner4_result mod 1409 and raw_p6 = Garner4_result mod 1601 re-encodes the aliased integer faithfully into the new lanes
  - Documents that the Garner4 hard precondition is NOT replaced but inherited by the 6p basis
  - Includes the decisive P5B-9A gate result proving the previous P5B-3 failure was a bug, not a mathematical impossibility
  - Provides the mathematical proof that per-prime NTT spectral computation in Z/1409Z and Z/1601Z correctly produces V mod p₅ and V mod p₆ where V is the true integer raw velocity

- `SESSION_*.md` - Session notes in `arena/` directory
  - SESSION_2026-04-27_D024.md - Documentation of the D024 probe session
  - SESSION_2026-04-27_PUSH7.md - Notes from the PUSH7 session
  - SESSION_2026-04-27_PUSH8.md - Notes from the PUSH8 session  
  - SESSION_2026-04-27_PUSH9.md - Notes from the PUSH9 session
  - SESSION_2026-04-28_PUSH10_v3.md - Final Push 10 session with Recumbent rule restoration
  - Documents the achievement of fully residue-pure NS step with bit-identical agreement to canonical solver
  - Details the progression from Push 7/8 (retracted) → Push 9 (baseline) → Push 10 (full residue purity)
  - Records the successful implementation of biot_savart_residue with raw spectral compute on 4 NTT primes
  - Documents the achievement of 0 mismatches in v3 step at amp=100 over 5-step trajectory

- `ns3d_n32_ntt6_P5B9A_prereqs.rs` - P5B-9A path prerequisites
  - Implements the 6-prime NTT extension with primes p5=1409 and p6=1601
  - Provides the mathematical basis for the extension with prime selection criteria
  - Includes the non-breaking design that keeps existing 4-prime arrays unchanged
  - Features additive new arrays with _6 suffix for the extended basis
  - Implements ntt3d_ext and intt3d_ext functions using the extended matrix for prime index 0..6
  - Provides garner6_centered_i128 reconstruction over all 6 primes
  - Ensures D-012 compliance with hardcoded ascending j-loop in Garner reconstruction
  - Includes comprehensive tests validating the 6-prime extension

- `landau_siegel_chimera_dag.md` - Landau-Siegel zero containment DAG
  - Details the corrected chimera lattice architecture for topological containment
  - Documents the 5-tuple chimera coordinates with zero-flags for divisor class discrimination
  - Specifies the ×4 closed cycle on (Z/pZ)× for L-function decomposition
  - Includes QR/QNR structural bias analysis with harmonic averaging
  - Provides loop invariance verification: |Forward - Reverse| = 0 for all tested D
  - Features the Double Chimera: Z/11²Z × Z/7²Z = Z/5929Z for combined constraint
  - Documents K-Elimination winding digit computation on Z/p²Z
  - Implements BSGS (Baby-Step Giant-Step) for multiplicative order computation
  - Shows QR positions have 1.83× harmonic average heavier than QNR positions

- `ramanujan_memory_lane_dag.md` - Ramanujan memory bridge DAG
  - Documents the cross-domain memory bridge via Ramanujan harmonic residues
  - Features 144-band WASSAN substrate for winding vector storage
  - Provides three-domain interpretation: CRAM/CRT, Fluid (NS), and Quantum
  - Includes shadow anchor lane (prime 11) that remains at MultidimensionalZero
  - Implements Permanent Non-Resonance Theorem: shadow lane never resonates with NS coefficients
  - Features spectral fingerprint bridge using sieve power SP(p,d) = (2p-1-c_p(d))/p²
  - Uses Ramanujan sums c_q(n) = q-1 if q|n, else -1 for harmonic computation
  - Includes HarmonicLane with operators (Add/Sub) and roles (Active/ShadowAnchor)
  - Implements band indexing: 28 bands per prime distributed across 144 bands

- `hydra_v3_dag.md` - Hydra sieve algorithm DAG
  - Details the multi-dimensional operator-toric carry-propagation sieve
  - Features heterogeneous optimal operators per prime and family
  - Includes K-Elimination magnitude oracle for exact reconstruction
  - Provides PLMG rail/void validator for O(1) geometric checks
  - Supports multiple prime families: twin, cousin, sexy, Sophie Germain, Goldbach, Polignac
  - Implements carry spectral fingerprinting for family characterization
  - Offers 79.8% elimination rate for twin primes with 0 false negatives
  - Includes operator separation tables for per-prime optimization
  - Features calibrated oracle atlas with 99.9% recall and 5.22× enrichment

- `mmbf_grover_dag.md` - Ramanujan-Grover bridge DAG
  - Documents the Ramanujan-Hydra Spectral Correspondence (T6) as an oracle
  - Provides Dense Toric Grover search engine for prime sieving
  - Uses K-Elimination for exact reconstruction
  - Features the Ramanujan-Grover formula: T_opt = ⌊π/4 · p / √(2p - 1 - c_p(d))⌋
  - Supports two-tier oracle: classical (p ∈ {2,5,13}) and Grover-admissible (p ∈ {3,7,11})
  - Provides 141× speedup for large primes (p~10^4) vs classical enumeration
  - Includes sieve power computation and forbidden residue classification

## Directory Structure

```
.
├── README.md
├── .gitignore
├── arena/
│   ├── Cargo.toml
│   ├── README.md
│   ├── SESSION_*.md                    ← per-session notes (D024, PUSH7-10)
│   ├── executioner_dag.md
│   ├── landau_siegel_chimera_dag.md    ← L-S zero containment architecture
│   ├── ramanujan_memory_lane_dag.md    ← Ramanujan memory bridge design
│   ├── hydra_v3_dag.md                 ← Hydra sieve algorithm specification
│   ├── mmbf_grover_dag.md              ← Ramanujan-Grover bridge design
│   │
│   ├── crates/                         ← System 2: NS Solver
│   │   ├── asve_engine/
│   │   ├── cdhs_telemetry/
│   │   ├── cram_explorer/              ← local solver-side crate
│   │   ├── d024_probe/
│   │   ├── nsgrid3d/
│   │   ├── ns_v5/
│   │   ├── ns_winding_v2/
│   │   ├── qcid_ns_bridge/
│   │   ├── qedde_v2/, qedde_v3/
│   │   ├── wassan_engine/, wassan_holographic/
│   │   └── ... (~25 crates)
│   │
│   ├── qmnf_unified/                   ← System 1: Mother Prime Sieve
│   │   ├── crates/                     ← sieve-side instances of the substrate
│   │   ├── docs/atlas/                 ← CRAM operator skill, dangerous configs
│   │   ├── docs/cram_definitive/       ← Algebraic Measurement Framework, atlas
│   │   ├── docs/wave3/                 ← theorem stacks, blueprints
│   │   ├── lean/                       ← Lean formalization seeds
│   │   ├── python/                     ← resnet, yukawa, hydra, separation
│   │   ├── skills/                     ← Claude Code skill bundles
│   │   └── tools/python/               ← sieve + analysis utilities
│   │
│   └── prime_hunt/                     ← top-level mirror of sieve crate
│
├── ns3d_n32_ntt6_P5B9A_prereqs.rs      ← P5B-9A path prerequisites
├── P5B_DAG_P5B9A_GATE.md               ← master DAG (corrected architecture)
├── PROPOSAL_P5B9A_EXECUTION.md         ← next-step proposal
└── QWEN.md                             ← comprehensive project documentation
```

## Additional Documentation

### Core Algorithm Specifications
- `P5B_DAG_P5B9A_GATE.md` - Main algorithmic architecture with the corrected 6-prime NTT implementation
- `PROPOSAL_P5B9A_EXECUTION.md` - Implementation roadmap and acceptance criteria for 6-prime spectral Biot-Savart
- `executioner_dag.md` - ASVE cognitive architecture and session execution plans
- `landau_siegel_chimera_dag.md` - Topological containment approach for Landau-Siegel zeros
- `ramanujan_memory_lane_dag.md` - Cross-domain memory bridge via Ramanujan harmonic residues
- `hydra_v3_dag.md` - Multi-dimensional sieve architecture with heterogeneous operators
- `mmbf_grover_dag.md` - Ramanujan-Grover quantum search bridge

### Session Logs
- `SESSION_2026-04-27_D024.md` - Diagnostic session for attractor and symmetry tests
- `SESSION_2026-04-27_PUSH7.md` - Early implementation session (retracted)
- `SESSION_2026-04-27_PUSH8.md` - Early implementation session (retracted)
- `SESSION_2026-04-27_PUSH9.md` - Baseline implementation session
- `SESSION_2026-04-28_PUSH10_v3.md` - Final implementation achieving full residue-purity

### Mathematical Frameworks
- `docs/atlas/` - CRAM operator skills and dangerous configurations
- `docs/cram_definitive/` - Algebraic Measurement Framework and operator atlas
- `docs/wave3/` - Theorem stacks and mathematical blueprints
- `lean/` - Lean formalization seeds for theorem verification
- `skills/` - Claude Code skill bundles for enhanced functionality

## Detailed Crate Information

### Core Solver Crates

#### ns_v5 (ns_regularity)
- Implements exact-integer Navier-Stokes regularity via CRT contraction
- Contains 24 proved theorems with honest negatives and open problems
- Features the contraction bound theorem: B(N) ≤ 90/1001 for all N
- Includes exact-rational arithmetic for all computations
- Provides discrete maximum principle verification
- Houses the 3D exact solver with exact 3D operator family
- Implements the Recumbent rule: decompose once at start, operate on residues throughout, reconstruct only at output
- Contains ns3d_n32, ns3d_n32_crt, cram_div, and ns3d_n32_ntt6 modules
- Features exact rational numbers for precise fractional computations

#### nsgrid3d
- Contains the core 3D Navier-Stokes operators
- Implements biot_savart with discrete Biot-Savart computation
- Provides fused_solver for integrated operator evaluation
- Features regularity_tracker for monitoring solution behavior
- Includes schema_sweep for systematic parameter exploration
- Implements dual_tray for dual-strand computation verification
- Contains schema_program for dynamic schema execution
- Features cram_ns_bridge for connecting different computational domains
- Implements sobolev_winding for Sobolev space analysis

#### cram_div
- Implements residue-pure exact integer division in extended CRT basis
- Provides CRAM (Compressed Representation and Arithmetic Method) for exact division
- Extends CRT basis with small-prime-power lanes (2^31, 5^9) to absorb divisors
- Supports exact division for divisors that are products of {2, 5} only
- Includes DivCtx for pre-computed per-divisor inverses to eliminate repeated modinv calls
- Features both truncating division (trunc_div_signed) and exact division (exact_div_signed_zero_remainder)
- Implements div_contract_partial_mod for computing partial modulus minimums
- Contains garner_n_centered_i128 for generic N-prime centred Garner reconstruction

### Holographic Storage Crates

#### wassan_holographic
- Implements 144-band NTT holographic storage
- Features 144 data bands and 112 parity bands for error correction
- Uses NTT transforms to create holographic sectors where each sector contains info about all coefficients
- Supports error correction with up to 56 band errors (112 parity bands total)
- Provides phase-locked retrieval via coherence matching
- Implements self-correction via attractor basin convergence
- Features zero-float architecture with exact modular integer arithmetic
- Includes holographic property: losing any single sector does not destroy data

#### wassan_engine
- Implements WASSAN band arithmetic and memory management
- Provides convergence tracking with syndrome detection and correction
- Manages attractor banks for phase-locked memory retrieval
- Tracks stability scores and correction statistics
- Features basin creation, storage and retrieval mechanisms
- Implements ConvergenceTracker with stores, retrievals, corrections statistics
- Provides syndrome detection for error correction

#### hdv_crt
- Implements Hyperdimensional CRT vectors
- Provides multi-dimensional CRT representations
- Supports high-dimensional modular arithmetic operations
- Features efficient residue computation across multiple dimensions
- Includes hyperdimensional storage and retrieval mechanisms

#### plmg_validator
- Implements Rail/Void phase-lock validation
- Provides exact rational ratios for precise probability calculations
- Features RailSignal enum for on-rail and in-void states
- Implements trajectory bound and void ratio calculations
- Supports confidence measurement in exact rational form

### Cognitive and Advanced Math Crates

#### asve_engine
- Implements Adaptive Self-Validating Engine (cognitive engine)
- Features cognitive reentry loop: PERCEIVE → REMEMBER → ANALYZE → HYPOTHESIZE → VALIDATE → LEARN → SECURE
- Provides integer-only cognitive state with quantized metrics (coherence, confidence, novelty, threat)
- Maintains knowledge graph with concepts, relations and validation
- Detects cognitive gaps and generates hypotheses to address them
- Includes paradigm shift mechanism for axiom suspension and re-evaluation
- Provides NLG hedge level based on coherence score
- Supports WASSAN integration for memory storage and retrieval

#### time_crystal
- Implements phi-resonant integer oscillator with Fibonacci aperiodic gating
- Provides temporal heartbeat for HackFate systems (Loki, CDHS, ASVE)
- Uses golden ratio approximation as integer fraction (PHI_NUM/PHI_DEN)
- Features Digital Time Crystal with oscillator and gate components
- Generates aperiodic gate events at Fibonacci-indexed tick counts
- Produces high-quality deterministic entropy for CT-DRBG
- Includes phase readout for health monitoring

#### svd_engine
- Implements integer-only SVD engine with power iteration for dominant singular value
- Features Jacobi rotations via integer CORDIC for full SVD of small matrices
- Computes coherence score as ratio of dominant singular value to Frobenius norm
- Provides cognitive interface for ASVE to measure knowledge graph coherence
- Uses scaled integer arithmetic (2^20 scale factor) for precision
- Includes HDV similarity matrix coherence computation
- Calculates effective rank estimation (Frobenius² / σ₁²)

### Search and Optimization Crates

#### mmbf_grover_bridge
- Implements the Ramanujan-Hydra Spectral Correspondence (T6) as an oracle
- Provides Dense Toric Grover search engine for prime sieving
- Uses K-Elimination for exact reconstruction
- Features the Ramanujan-Grover formula: T_opt = ⌊π/4 · p / √(2p - 1 - c_p(d))⌋
- Supports two-tier oracle: classical (p ∈ {2,5,13}) and Grover-admissible (p ∈ {3,7,11})
- Provides 141× speedup for large primes (p~10^4) vs classical enumeration
- Includes sieve power computation and forbidden residue classification

#### qedde_v2
- Implements interchangeable schema cell architecture with hot-swappable computation units
- Features CompositeSchema for chaining multiple schemas in sequence
- Includes PhaseGate for inter-schema handoff with integrity checks
- Provides NS-specific schemas: turbulence, vortex, multi-scale, and regularity analysis
- Contains Kähler/Picard-Fuchs schemas for geometric invariant computation
- Offers schema factory for building complete pipelines (NS, Yukawa-Kähler, turbulence)
- Implements OracleA/B for structural exclusion and objective scoring
- Supports diagnostic sinks for winding/entropy/residual tracking

#### qedde_v3
- Implements unified substrate architecture integrating QEDDE, HackFate, and MMBF
- Features WASSAN drive as shared storage substrate for all operations
- Implements Double Helix verifier for every write/read operation
- Provides Attractor Memory cells with self-correction capabilities
- Includes resident oracles (Grover, Ramanujan, Shor) available to all cells
- Features CRT Beacon as shared accelerator for K-Elimination and reconstruction
- Implements Shadow Entropy pool for tamper-resistant operation
- Supports multiple cell types: PrimeSieve, NSVelocity, NSVortex, NeuralForward, etc.
- Provides fold function for irreversible destruction when tamper detected
- Uses Fibonacci-scheduled verification phases
- Implements Descartes phase-lock verification across four sectors

#### hydra_v3
- Implements multi-dimensional operator-toric carry-propagation sieve
- Features heterogeneous optimal operators per prime and family
- Includes K-Elimination magnitude oracle for exact reconstruction
- Provides PLMG rail/void validator for O(1) geometric checks
- Supports multiple prime families: twin, cousin, sexy, Sophie Germain, Goldbach, Polignac
- Implements carry spectral fingerprinting for family characterization
- Offers 79.8% elimination rate for twin primes with 0 false negatives
- Includes operator separation tables for per-prime optimization
- Features calibrated oracle atlas with 99.9% recall and 5.22× enrichment

### Specialized Mathematics Crates

#### k_elim_period
- Implements BSGS (Baby-Step Giant-Step) algorithm for multiplicative order computation
- Fixes the broken doubling strategy that only found periods that are powers of 2
- Provides O(√N) time complexity for period finding instead of exponential
- Features period verification to ensure minimality and correctness
- Implements batch period finder for all CRT lanes simultaneously
- Computes winding period of CRT torus orbit where k=0 (energy neutral)
- Achieves 106-232× speedup over naive methods for large primes
- Includes safe basis lane testing across all primes {2,3,5,7,11,13}

#### landau_siegel_chimera
- Implements topological containment of Landau-Siegel zeros via corrected chimera lattice
- Features 5-tuple chimera coordinates with zero-flags for divisor class discrimination
- Provides ×4 closed cycle on (Z/pZ)× for L-function decomposition
- Implements QR/QNR structural bias analysis with harmonic averaging
- Features loop invariance: |Forward - Reverse| = 0 for all tested D
- Includes Double Chimera: Z/11²Z × Z/7²Z = Z/5929Z for combined constraint
- Provides K-Elimination winding digit computation on Z/p²Z
- Implements BSGS (Baby-Step Giant-Step) for multiplicative order computation
- Shows QR positions have 1.83× harmonic average heavier than QNR positions

#### quantum_continuum_torus
- Implements Quantum Continuum Injected Discrete Torus (QCIDT) architecture
- Features F_p² quantum amplitude representation (a + b√δ) with Frobenius automorphism
- Provides discrete readout via norm: N(a+b√δ) = a²-δb² (Frobenius-invariant)
- Includes winding correction engine for prime detection with arrow of time computation
- Implements resolution ladder with inter-resolution contraction injection
- Features rung-aware sieve with 15-lane configuration achieving 90%+ precision
- Incorporates Maya calendar-inspired chimera engine with Tzolkin, Haab, and Round layers
- Provides K-Elimination bridge between quantum and discrete layers
- Includes Copernican frame with observer-dependent derotation
- Implements shadow-gated injection mechanism tied to prime 11 resonance

#### ramanujan_memory_lane
- Implements cross-domain memory bridge via Ramanujan harmonic residues
- Features 144-band WASSAN substrate for winding vector storage
- Provides three-domain interpretation: CRAM/CRT, Fluid (NS), and Quantum
- Includes shadow anchor lane (prime 11) that remains at MultidimensionalZero
- Implements Permanent Non-Resonance Theorem: shadow lane never resonates with NS coefficients
- Features spectral fingerprint bridge using sieve power SP(p,d) = (2p-1-c_p(d))/p²
- Uses Ramanujan sums c_q(n) = q-1 if q|n, else -1 for harmonic computation
- Includes HarmonicLane with operators (Add/Sub) and roles (Active/ShadowAnchor)
- Implements band indexing: 28 bands per prime distributed across 144 bands

#### prime_hunt
- Implements coprimality-first prime detection algorithms
- Features shadow gap sieve for identifying primes by finding residues that escape shadow network coverage
- Includes admissible sieve that walks only coprime residue classes
- Provides K-Elimination winding analysis for composite detection
- Offers cousin pair (gap-4) detection with spectral analysis
- Includes displacement power analysis for tracking missing primes in periods
- Implements K-Vort accumulator for winding number evolution patterns
- Uses SAFE_BASIS {2,3,5,7,11,13} for residue-based classification

### Supporting Crates

#### qmnf_primitives
- Provides single source of truth for modular arithmetic, NTT, K-Elimination, Garner reconstruction, and Residue types
- Implements modular arithmetic primitives (addmod, submod, mulmod)
- Features NTT engine with per-lane context
- Includes Residues type for multi-lane representation
- Supports K-Elimination and Garner reconstruction
- Provides 13-smooth factorization and safe anchor mechanisms

#### qmnf_crtbigint
- Implements extended CRT BigInt implementation with seven-layer exact computation architecture
- Features modular primitives (mulmod, gcd, mod_inverse)
- Provides exact CRT lanes: 8×63-bit primes (~504-bit capacity)
- Includes safe basis composites: {2,3,5,7,11,13} phase-locked sub-lanes
- Implements safe anchor: single large prime for O(1) magnitude recovery
- Features WASSAN bands: 144 φ-harmonic execution lanes
- Provides PLMG rails/voids: 28% valid / 72% free error detection
- Implements Double Helix: dual-strand compute + verify
- Includes exact rational numbers, integer sine approximations, and safe anchor mechanisms

#### d024_probe
- Contains diagnostic and probe tools for NS analysis
- Includes attractor tests, K_b symmetry tests, b-component attractor tests
- Provides corridor probe tools (corridor_probe_v3) for measuring computational limits
- Implements attractor basin analysis for stability assessment
- Features K_b symmetry tests for solution verification
- Supports b-component attractor tests for behavior analysis

#### cram_explorer
- Provides tools for exploring mathematical structures (Apollonian, holodrive, k_elim, projector, shadow_ladder, torus)
- Implements Apollonian circle packing algorithms
- Features holodrive for high-dimensional visualization
- Includes K-Elimination analysis tools
- Provides projector for dimensional reduction
- Implements shadow_ladder for hierarchical analysis
- Supports torus-based computations

#### qcid_ns_bridge
- Implements Fp² dual-track NS bridge for connecting different computational domains
- Provides Fp2 elements (a + b√δ) with physics on residue (a) and continuum info on amplitude (b)
- Features dual-track state management with independent physics and contraction tracking
- Includes norm computation (a² - δb²) as the bridge quantity between domains
- Supports Frobenius conjugate (σ(a+b√δ) = a-b√δ) for amplitude flipping while preserving residue
- Contains ring homomorphism properties for integer embedding in Fp2
- Supports nested radical computations for N=16 and N=32 FFTs

#### maa_dh
- Implements Apollonian Double Helix (AHOP post-quantum cryptography)
- Features Mutual Apollonian Attestation with dual CRT strands
- Uses Descartes Circle Theorem as invariant for verification: (a+b+c+d)² = 2(a²+b²+c²+d²)
- Provides dual-strand computation with checkpoint-based verification
- Implements corruption detection via strand disagreement
- Generates Apollonian tree structures for attestation
- Includes integer square root computation for Descartes verification
- Supports post-quantum cryptographic applications through Apollonian packings

#### cdhs_telemetry
- Implements health diagnostics for the system
- Provides real-time monitoring of computational health
- Features anomaly detection for early issue identification
- Includes performance metrics tracking
- Supports system stability assessment
- Implements alerting mechanisms for critical issues

#### loki_engine
- Provides threat detection for the system
- Implements security monitoring and intrusion detection
- Features anomaly analysis for suspicious activities
- Includes tamper detection mechanisms
- Supports cryptographic key management
- Provides security audit trails

#### hackfate_runtime
- Implements full lifecycle runtime for the system
- Provides initialization and shutdown procedures
- Features resource management and allocation
- Includes error handling and recovery mechanisms
- Supports configuration management
- Implements logging and monitoring infrastructure

#### sanity_crt
- Implements CRT sanity checking tools
- Provides validation for CRT computations
- Features consistency checks across residue lanes
- Includes error detection and correction mechanisms
- Supports verification of CRT operations
- Implements diagnostic utilities for CRT computations
```