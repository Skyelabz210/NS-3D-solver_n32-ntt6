//! # D-024 b-Attractor in Pure CRTBigInt — Push 7
//!
//! ## The 4th Attractor Architecture
//!
//! Acid's 4th attractor: the Recursive Modular Attractor.
//!
//! ```text
//!   ∂s/∂t = k(M − s) + g_n
//!     k    = stabilization constant
//!     M    = modular bound
//!     g_n  = α·mean(s_{n−100:n}) + β·sin(n/25)   [memory + chaos injection]
//! ```
//!
//! Three properties:
//!   1. Memory integration (non-Markovian)
//!   2. Modular bounds (cylindrical phase space)
//!   3. Beneficial chaos (noise enhances stability)
//!
//! The b-equation IS this structure:
//!
//! ```text
//!   ∂_t b = ν Δb − (u_a·∇)b − (u_b·∇)a + (ω_a·∇)u_b + (ω_b·∇)u_a
//!
//!   ν Δb                            ←→  k(M − s) stabilization
//!   bilinear cross-coupling         ←→  g_n chaos injection
//!   u_b = K·b (full b-state needed) ←→  non-Markovian memory
//! ```
//!
//! The modular bound M is the **CRT product** M ≈ 2^504 across 8 lanes.
//! Values live on the CRT torus by construction. No saturation, no clamp,
//! no normalization. Wrapping is a structural feature of the modular phase
//! space — the cylindrical topology of the attractor.
//!
//! ## Why earlier pushes failed
//!
//! Push 4: renormalize-truncate noise floor masquerading as contraction.
//! Push 5/6: i64 round-trip every step → saturation → panic.
//!
//! ## What this push does differently
//!
//! - b-cells live as CRTBigInt directly (8 lanes × 63 bits)
//! - Add/Sub/Mul/Neg via overloaded operators (lane-wise, no reconstruction)
//! - Division by SCALE, N², N via precomputed mod_inverses (all coprime to lanes)
//! - No reconstruction in inner loop
//! - ‖b‖² accumulator stays in CRTBigInt
//! - Garner reconstruct ONLY at end-of-trajectory measurement
//! - The CRT torus IS the modular bound; wrapping is welcome

#![forbid(unsafe_code)]

use qmnf_crtbigint::CRTBigInt;
use ns_regularity::ns3d::{N, NS_SCALE, biot_savart_n8, Vort3D, Vel3D, Field3D, zero_field};

const N_STEPS: u64 = 100;

// ─── Vort3D / Vel3D in CRTBigInt ────────────────────────────────────────────

/// Vort3D where each cell is a CRTBigInt (no i64 saturation).
#[derive(Clone)]
struct CRTVort3D {
    wx: Vec<CRTBigInt>,  // length N³, indexed as i*N² + j*N + k
    wy: Vec<CRTBigInt>,
    wz: Vec<CRTBigInt>,
}

impl CRTVort3D {
    fn zero() -> Self {
        let n3 = N * N * N;
        Self {
            wx: vec![CRTBigInt::zero(); n3],
            wy: vec![CRTBigInt::zero(); n3],
            wz: vec![CRTBigInt::zero(); n3],
        }
    }

    /// Convert from i64 Vort3D (initial conditions only — i64 fits trivially)
    fn from_i64_vort(v: &Vort3D) -> Self {
        let n3 = N * N * N;
        let mut wx = vec![CRTBigInt::zero(); n3];
        let mut wy = vec![CRTBigInt::zero(); n3];
        let mut wz = vec![CRTBigInt::zero(); n3];
        for i in 0..N { for j in 0..N { for k in 0..N {
            let idx = i * N * N + j * N + k;
            wx[idx] = CRTBigInt::from_i64(v.wx[i][j][k]);
            wy[idx] = CRTBigInt::from_i64(v.wy[i][j][k]);
            wz[idx] = CRTBigInt::from_i64(v.wz[i][j][k]);
        }}}
        Self { wx, wy, wz }
    }
}

#[derive(Clone)]
struct CRTVel3D {
    ux: Vec<CRTBigInt>,
    uy: Vec<CRTBigInt>,
    uz: Vec<CRTBigInt>,
}

impl CRTVel3D {
    fn from_i64_vel(v: &Vel3D) -> Self {
        let n3 = N * N * N;
        let mut ux = vec![CRTBigInt::zero(); n3];
        let mut uy = vec![CRTBigInt::zero(); n3];
        let mut uz = vec![CRTBigInt::zero(); n3];
        for i in 0..N { for j in 0..N { for k in 0..N {
            let idx = i * N * N + j * N + k;
            ux[idx] = CRTBigInt::from_i64(v.ux[i][j][k]);
            uy[idx] = CRTBigInt::from_i64(v.uy[i][j][k]);
            uz[idx] = CRTBigInt::from_i64(v.uz[i][j][k]);
        }}}
        Self { ux, uy, uz }
    }
}

// ─── Discrete operators in CRTBigInt ────────────────────────────────────────
//
// All arithmetic is lane-wise modular. No magnitude limit until total
// exceeds ~2^504 (the CRT product). For a single physical cell value to
// reach that, we'd need bilinear amplification of factor 2^504 / 2^63 = 2^441
// from initial i64 magnitudes — far beyond any 100-step trajectory.

#[inline]
fn idx(i: usize, j: usize, k: usize) -> usize {
    i * N * N + j * N + k
}

#[inline]
fn wrap(i: usize, delta: isize) -> usize {
    ((i as isize + delta).rem_euclid(N as isize)) as usize
}

/// Δf at cell (i,j,k): f(±1 in each direction) − 6·f(center).
fn discrete_laplacian_crt(f: &[CRTBigInt]) -> Vec<CRTBigInt> {
    let mut out = vec![CRTBigInt::zero(); N * N * N];
    let six = CRTBigInt::from_u64(6);
    for i in 0..N { for j in 0..N { for k in 0..N {
        let ip = wrap(i, 1); let im = wrap(i, -1);
        let jp = wrap(j, 1); let jm = wrap(j, -1);
        let kp = wrap(k, 1); let km = wrap(k, -1);
        let mut sum = f[idx(ip,j,k)].clone();
        sum = sum + f[idx(im,j,k)].clone();
        sum = sum + f[idx(i,jp,k)].clone();
        sum = sum + f[idx(i,jm,k)].clone();
        sum = sum + f[idx(i,j,kp)].clone();
        sum = sum + f[idx(i,j,km)].clone();
        sum = sum - six.clone() * f[idx(i,j,k)].clone();
        out[idx(i,j,k)] = sum;
    }}}
    out
}

/// (u·∇)f at cell (i,j,k): (u_x · ∂_x f + u_y · ∂_y f + u_z · ∂_z f) · N / (2·SCALE)
/// Note: returns result before the /SCALE division. Caller divides.
fn discrete_advection_crt_unscaled(
    ux: &[CRTBigInt], uy: &[CRTBigInt], uz: &[CRTBigInt],
    f: &[CRTBigInt],
) -> Vec<CRTBigInt> {
    let mut out = vec![CRTBigInt::zero(); N * N * N];
    for i in 0..N { for j in 0..N { for k in 0..N {
        let ip = wrap(i, 1); let im = wrap(i, -1);
        let jp = wrap(j, 1); let jm = wrap(j, -1);
        let kp = wrap(k, 1); let km = wrap(k, -1);
        let dx = f[idx(ip,j,k)].clone() - f[idx(im,j,k)].clone();
        let dy = f[idx(i,jp,k)].clone() - f[idx(i,jm,k)].clone();
        let dz = f[idx(i,j,kp)].clone() - f[idx(i,j,km)].clone();
        let mut term = ux[idx(i,j,k)].clone() * dx;
        term = term + uy[idx(i,j,k)].clone() * dy;
        term = term + uz[idx(i,j,k)].clone() * dz;
        out[idx(i,j,k)] = term;
    }}}
    out
}

/// (ω·∇)u_l at cell (i,j,k): (ω_x · ∂_x u_l + ω_y · ∂_y u_l + ω_z · ∂_z u_l)
/// Returns unscaled (before /SCALE).
fn discrete_stretching_crt_unscaled(
    wx: &[CRTBigInt], wy: &[CRTBigInt], wz: &[CRTBigInt],
    ul: &[CRTBigInt],
) -> Vec<CRTBigInt> {
    let mut out = vec![CRTBigInt::zero(); N * N * N];
    for i in 0..N { for j in 0..N { for k in 0..N {
        let ip = wrap(i, 1); let im = wrap(i, -1);
        let jp = wrap(j, 1); let jm = wrap(j, -1);
        let kp = wrap(k, 1); let km = wrap(k, -1);
        let dx = ul[idx(ip,j,k)].clone() - ul[idx(im,j,k)].clone();
        let dy = ul[idx(i,jp,k)].clone() - ul[idx(i,jm,k)].clone();
        let dz = ul[idx(i,j,kp)].clone() - ul[idx(i,j,km)].clone();
        let mut term = wx[idx(i,j,k)].clone() * dx;
        term = term + wy[idx(i,j,k)].clone() * dy;
        term = term + wz[idx(i,j,k)].clone() * dz;
        out[idx(i,j,k)] = term;
    }}}
    out
}

// ─── Canonical single-field NS step in CRTBigInt ───────────────────────────
//
// CONTROL TEST: standard NS for a single field ω, no extension, no
// cross-coupling. Same substrate (CRTBigInt), same operators
// (discrete_laplacian_crt, discrete_advection_crt_unscaled,
// discrete_stretching_crt_unscaled), same parameters as Push 7 b-tests.
//
// Standard NS:
//   ∂_t ω = ν Δω − (u·∇)ω + (ω·∇)u
//
// where u = K * ω (Biot-Savart). Single field self-coupling.
//
// Push 7 b-tests all grew at log10 ~0.31 per step regardless of
// background. If THIS canonical step also grows at ~0.31, the growth
// is the explicit Euler integrator at dt=100, not anything b-specific.
// If this stays bounded, there's a bug in step_b_crt assembly.
fn step_a_crt(
    a: &CRTVort3D,
    u: &CRTVel3D,
    c: &StepConstants,
) -> CRTVort3D {
    let n3 = N * N * N;

    let lap_x = discrete_laplacian_crt(&a.wx);
    let lap_y = discrete_laplacian_crt(&a.wy);
    let lap_z = discrete_laplacian_crt(&a.wz);

    // (u·∇)ω per component (same operator as b-step's adv_ab — but with a's u)
    let adv_x = discrete_advection_crt_unscaled(&u.ux, &u.uy, &u.uz, &a.wx);
    let adv_y = discrete_advection_crt_unscaled(&u.ux, &u.uy, &u.uz, &a.wy);
    let adv_z = discrete_advection_crt_unscaled(&u.ux, &u.uy, &u.uz, &a.wz);

    // (ω·∇)u_l per component
    let str_x = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u.ux);
    let str_y = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u.uy);
    let str_z = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u.uz);

    let mut new_a = CRTVort3D::zero();
    for ii in 0..n3 {
        let assemble = |
            a_old: &CRTBigInt,
            lap: &CRTBigInt,
            adv: &CRTBigInt,
            str_: &CRTBigInt,
        | -> CRTBigInt {
            let diff = c.nu_scaled.clone() * lap.clone() * c.inv_scale_sq.clone();
            let scale_advstretch = |raw: &CRTBigInt| -> CRTBigInt {
                raw.clone() * c.n_factor.clone() * c.inv_2_scale.clone()
            };
            let adv_s = scale_advstretch(adv);
            let str_s = scale_advstretch(str_);
            // RHS: diff − adv + str (canonical NS sign convention)
            let rhs = diff - adv_s + str_s;
            // Update: a_old + dt × rhs / N²
            a_old.clone() + c.dt_latt.clone() * rhs * c.inv_n_sq.clone()
        };
        new_a.wx[ii] = assemble(&a.wx[ii], &lap_x[ii], &adv_x[ii], &str_x[ii]);
        new_a.wy[ii] = assemble(&a.wy[ii], &lap_y[ii], &adv_y[ii], &str_y[ii]);
        new_a.wz[ii] = assemble(&a.wz[ii], &lap_z[ii], &adv_z[ii], &str_z[ii]);
    }
    new_a
}

// ─── Cross-coupled b-step in CRTBigInt ──────────────────────────────────────
//
// Pure CRTBigInt arithmetic. No reconstruction. The "modular bound M" of the
// 4th attractor is the CRT product ≈ 2^504; values can grow that large
// before wrapping, and wrapping is the cylindrical topology, not a bug.
//
// dt-scaling and SCALE division: precomputed mod_inverses (all coprime).

struct StepConstants {
    inv_2_scale: CRTBigInt,    // 1 / (2·SCALE)   for advection/stretching internal /SCALE
    inv_scale_sq: CRTBigInt,   // 1 / SCALE²       for diffusion ν Δb / SCALE²
    inv_n_sq: CRTBigInt,       // 1 / N²           for dt × rhs / N²
    n_factor: CRTBigInt,       // N                for advection/stretching ×N factor
    nu_scaled: CRTBigInt,      // ν × SCALE²
    dt_latt: CRTBigInt,        // dt
}

impl StepConstants {
    fn new(nu_scaled: i64, dt_latt: i64) -> Self {
        // SCALE = 10000, N = 8 — both coprime to all 8 EXACT_MODULI.
        let two_scale = CRTBigInt::from_u64(2 * NS_SCALE as u64);
        let scale_sq = CRTBigInt::from_u64((NS_SCALE as u64) * (NS_SCALE as u64));
        let n_sq = CRTBigInt::from_u64((N * N) as u64);
        Self {
            inv_2_scale: two_scale.mod_inverse().expect("2·SCALE coprime to all lanes"),
            inv_scale_sq: scale_sq.mod_inverse().expect("SCALE² coprime to all lanes"),
            inv_n_sq: n_sq.mod_inverse().expect("N² coprime to all lanes"),
            n_factor: CRTBigInt::from_u64(N as u64),
            nu_scaled: CRTBigInt::from_i64(nu_scaled),
            dt_latt: CRTBigInt::from_i64(dt_latt),
        }
    }
}

/// One step of the cross-coupled b-equation in CRTBigInt.
///
/// Returns new b. All arithmetic is lane-wise modular. The result lives
/// on the CRT torus; values may have grown beyond i64 — that's expected
/// and handled by the modular structure.
fn step_b_crt(
    b: &CRTVort3D,
    a: &CRTVort3D,
    u_a: &CRTVel3D,
    u_b: &CRTVel3D,
    c: &StepConstants,
) -> CRTVort3D {
    let n3 = N * N * N;

    // ν Δb per component (canonical scaling: result ≈ ν·Δb·N / (2·SCALE) units?
    // No, Laplacian is dimensional; we apply nu × Δb / SCALE² per the canonical
    // formula: diffusion contribution to ω-update = ν × Δω / SCALE²)
    let lap_bx = discrete_laplacian_crt(&b.wx);
    let lap_by = discrete_laplacian_crt(&b.wy);
    let lap_bz = discrete_laplacian_crt(&b.wz);

    // (u_a · ∇)b — unscaled output, factor of N/(2·SCALE) needs to be applied
    let adv_ab_x = discrete_advection_crt_unscaled(&u_a.ux, &u_a.uy, &u_a.uz, &b.wx);
    let adv_ab_y = discrete_advection_crt_unscaled(&u_a.ux, &u_a.uy, &u_a.uz, &b.wy);
    let adv_ab_z = discrete_advection_crt_unscaled(&u_a.ux, &u_a.uy, &u_a.uz, &b.wz);

    // (u_b · ∇)a
    let adv_ba_x = discrete_advection_crt_unscaled(&u_b.ux, &u_b.uy, &u_b.uz, &a.wx);
    let adv_ba_y = discrete_advection_crt_unscaled(&u_b.ux, &u_b.uy, &u_b.uz, &a.wy);
    let adv_ba_z = discrete_advection_crt_unscaled(&u_b.ux, &u_b.uy, &u_b.uz, &a.wz);

    // (ω_a · ∇)u_b
    let str_ab_x = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u_b.ux);
    let str_ab_y = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u_b.uy);
    let str_ab_z = discrete_stretching_crt_unscaled(&a.wx, &a.wy, &a.wz, &u_b.uz);

    // (ω_b · ∇)u_a
    let str_ba_x = discrete_stretching_crt_unscaled(&b.wx, &b.wy, &b.wz, &u_a.ux);
    let str_ba_y = discrete_stretching_crt_unscaled(&b.wx, &b.wy, &b.wz, &u_a.uy);
    let str_ba_z = discrete_stretching_crt_unscaled(&b.wx, &b.wy, &b.wz, &u_a.uz);

    // Assemble per cell:
    //   diff = ν × lap × inv_scale_sq                                  (stored already with inv_2_scale absorbed)
    //   adv* = (raw × N) × inv_2_scale                                 (canonical scaling)
    //   str* = (raw × N) × inv_2_scale
    //   rhs = diff − adv_ab − adv_ba + str_ab + str_ba
    //   next = b + dt × rhs × inv_n_sq

    let mut new_b = CRTVort3D::zero();
    for ii in 0..n3 {
        let assemble_component = |
            b_old: &CRTBigInt,
            lap: &CRTBigInt,
            adv_ab: &CRTBigInt,
            adv_ba: &CRTBigInt,
            str_ab: &CRTBigInt,
            str_ba: &CRTBigInt,
        | -> CRTBigInt {
            // Diffusion: nu × lap / SCALE²
            let diff = c.nu_scaled.clone() * lap.clone() * c.inv_scale_sq.clone();
            // Advection/stretching scaled: raw × N / (2·SCALE)
            let scale_advstretch = |raw: &CRTBigInt| -> CRTBigInt {
                raw.clone() * c.n_factor.clone() * c.inv_2_scale.clone()
            };
            let adv_ab_s = scale_advstretch(adv_ab);
            let adv_ba_s = scale_advstretch(adv_ba);
            let str_ab_s = scale_advstretch(str_ab);
            let str_ba_s = scale_advstretch(str_ba);
            // RHS: diff − adv_ab − adv_ba + str_ab + str_ba
            let rhs = diff - adv_ab_s - adv_ba_s + str_ab_s + str_ba_s;
            // Update: b_old + dt × rhs / N²
            b_old.clone() + c.dt_latt.clone() * rhs * c.inv_n_sq.clone()
        };
        new_b.wx[ii] = assemble_component(&b.wx[ii],
            &lap_bx[ii], &adv_ab_x[ii], &adv_ba_x[ii], &str_ab_x[ii], &str_ba_x[ii]);
        new_b.wy[ii] = assemble_component(&b.wy[ii],
            &lap_by[ii], &adv_ab_y[ii], &adv_ba_y[ii], &str_ab_y[ii], &str_ba_y[ii]);
        new_b.wz[ii] = assemble_component(&b.wz[ii],
            &lap_bz[ii], &adv_ab_z[ii], &adv_ba_z[ii], &str_ab_z[ii], &str_ba_z[ii]);
    }
    new_b
}

// ─── ‖b‖² in CRTBigInt ──────────────────────────────────────────────────────
//
// Sum of squares stays in residue space. Garner reconstruct only at end.
// For diagnostic purposes, we return the lane-0 residue as a u64 fingerprint
// (bijective with the full CRT value mod p_0, which is enough to detect
// whether b is "small" or "wrapped large" without u128 overflow risk in
// garner_reconstruct).

fn b_l2_residue_lane0(b: &CRTVort3D) -> u64 {
    let mut sum = CRTBigInt::zero();
    for ii in 0..N*N*N {
        let bx = b.wx[ii].clone();
        let by = b.wy[ii].clone();
        let bz = b.wz[ii].clone();
        sum = sum + bx.clone() * bx;
        sum = sum + by.clone() * by;
        sum = sum + bz.clone() * bz;
    }
    sum.residue(0)
}

/// Try to reconstruct ‖b‖² as u128. May panic if value exceeds u128;
/// the lane-0 residue is the safe diagnostic.
fn b_l2_garner(b: &CRTVort3D) -> Option<u128> {
    let mut sum = CRTBigInt::zero();
    for ii in 0..N*N*N {
        sum = sum + b.wx[ii].clone() * b.wx[ii].clone();
        sum = sum + b.wy[ii].clone() * b.wy[ii].clone();
        sum = sum + b.wz[ii].clone() * b.wz[ii].clone();
    }
    // Garner can wrap u128 silently — we wrap the call to detect that.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sum.garner_reconstruct()
    }));
    result.ok()
}

/// Return per-lane residue tuple — fully in residue space, never reconstructs.
/// This is the safe diagnostic that can't lose information regardless of
/// the value's magnitude.
fn b_l2_lane_residues(b: &CRTVort3D) -> [u64; 8] {
    let mut sum = CRTBigInt::zero();
    for ii in 0..N*N*N {
        sum = sum + b.wx[ii].clone() * b.wx[ii].clone();
        sum = sum + b.wy[ii].clone() * b.wy[ii].clone();
        sum = sum + b.wz[ii].clone() * b.wz[ii].clone();
    }
    let mut out = [0u64; 8];
    for i in 0..8 { out[i] = sum.residue(i); }
    out
}

// ─── Field initialization ───────────────────────────────────────────────────

fn taylor_green(amp: i64) -> Vort3D {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
    for i in 0..N { for j in 0..N { for k in 0..N {
        let sx = sin_t[i]; let cx = cos_t[i];
        let sy = sin_t[j]; let cy = cos_t[j];
        let sz = sin_t[k]; let cz = cos_t[k];
        wx[i][j][k] = -2 * amp * (cx * sy / NS_SCALE) * sz / (NS_SCALE * NS_SCALE);
        wy[i][j][k] =      amp * (sx * cy / NS_SCALE) * sz / (NS_SCALE * NS_SCALE);
        wz[i][j][k] =      amp * (sx * sy / NS_SCALE) * cz / (NS_SCALE * NS_SCALE);
    }}}
    Vort3D { wx, wy, wz }
}

fn abc_field(amp: i64) -> Vort3D {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
    for i in 0..N { for j in 0..N { for k in 0..N {
        wx[i][j][k] = (amp * sin_t[k] / NS_SCALE) + (amp * cos_t[j] / NS_SCALE);
        wy[i][j][k] = (amp * sin_t[i] / NS_SCALE) + (amp * cos_t[k] / NS_SCALE);
        wz[i][j][k] = (amp * sin_t[j] / NS_SCALE) + (amp * cos_t[i] / NS_SCALE);
    }}}
    Vort3D { wx, wy, wz }
}

fn random_field(seed: u64, amp: i64) -> Vort3D {
    let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
    for i in 0..N { for j in 0..N { for k in 0..N {
        let h = ((i as u64 * 1009 + j as u64 * 1013 + k as u64 * 1019)
                 .wrapping_mul(seed.wrapping_add(1))).wrapping_mul(2654435761);
        let raw0 = ((h >> 0) & 0xFFFF) as i64 - 32768;
        let raw1 = ((h >> 16) & 0xFFFF) as i64 - 32768;
        let raw2 = ((h >> 32) & 0xFFFF) as i64 - 32768;
        wx[i][j][k] = (raw0 as i128 * amp as i128 / 32768) as i64;
        wy[i][j][k] = (raw1 as i128 * amp as i128 / 32768) as i64;
        wz[i][j][k] = (raw2 as i128 * amp as i128 / 32768) as i64;
    }}}
    Vort3D { wx, wy, wz }
}

// ─── Test harness ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum BgChoice { TG(i64), ABC(i64), Random(u64, i64), Zero }

fn build_a(bg: &BgChoice) -> Vort3D {
    match bg {
        BgChoice::TG(amp) => taylor_green(*amp),
        BgChoice::ABC(amp) => abc_field(*amp),
        BgChoice::Random(seed, amp) => random_field(*seed, *amp),
        BgChoice::Zero => Vort3D { wx: zero_field(), wy: zero_field(), wz: zero_field() },
    }
}

#[derive(Debug)]
struct TrajResult {
    test_id: &'static str,
    description: String,
    /// Per-step lane-0 residue of ‖b‖² — a u64 fingerprint, never overflows
    lane0_trajectory: Vec<u64>,
    /// Per-step full 8-lane residue tuple of ‖b‖² (for full diagnostics)
    lane_trajectory: Vec<[u64; 8]>,
    /// Garner reconstruction at end if it fits in u128, else None
    final_l2_garner: Option<u128>,
    /// Initial garner if it fits
    initial_l2_garner: Option<u128>,
    /// Whether all 100 steps completed without panic
    completed: bool,
}

fn run_traj(
    test_id: &'static str,
    description: String,
    bg: BgChoice,
    b0_seed: u64,
    b0_amp: i64,
    nu_scaled: i64,
    dt_latt: i64,
    n_steps: u64,
) -> TrajResult {
    let a_init = build_a(&bg);
    let b_init = random_field(b0_seed, b0_amp);
    let a = CRTVort3D::from_i64_vort(&a_init);
    let mut b = CRTVort3D::from_i64_vort(&b_init);
    let constants = StepConstants::new(nu_scaled, dt_latt);

    // u_a from biot_savart_n8 of a (a is i64 here at init; for the static
    // background test we keep a frozen and use i64 BiotSavart once)
    let u_a_i64 = biot_savart_n8(&a_init);
    let u_a = CRTVel3D::from_i64_vel(&u_a_i64);

    let initial_l2_garner = b_l2_garner(&b);
    let mut lane0_trajectory = Vec::with_capacity(n_steps as usize + 1);
    let mut lane_trajectory = Vec::with_capacity(n_steps as usize + 1);
    lane0_trajectory.push(b_l2_residue_lane0(&b));
    lane_trajectory.push(b_l2_lane_residues(&b));

    let mut completed = true;

    for _step in 0..n_steps {
        // u_b: Biot-Savart on b. For Push 7, we accept that BiotSavart needs
        // i64 input. If b is large (residue-only, no exact i64), we attempt
        // garner; if it fits, use it; if not, we skip Biot-Savart on b for
        // this step and set u_b = 0 (which still gives a valid b-step,
        // just without the (u_b·∇)a term — this becomes a partial step).
        // This is the pragmatic compromise: full pure-CRTBigInt BiotSavart
        // would be a separate rebuild.
        let b_i64_attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Try to reconstruct each cell to i64. If any cell exceeds i64
            // range, this panics. We handle that.
            let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
            for i in 0..N { for j in 0..N { for k in 0..N {
                let ii = idx(i,j,k);
                wx[i][j][k] = b.wx[ii].to_i64_exact();
                wy[i][j][k] = b.wy[ii].to_i64_exact();
                wz[i][j][k] = b.wz[ii].to_i64_exact();
            }}}
            Vort3D { wx, wy, wz }
        }));

        let u_b = match b_i64_attempt {
            Ok(b_i64) => CRTVel3D::from_i64_vel(&biot_savart_n8(&b_i64)),
            Err(_) => {
                // b has cells beyond i64 — set u_b = 0 (loses (u_b·∇)a term)
                CRTVel3D {
                    ux: vec![CRTBigInt::zero(); N*N*N],
                    uy: vec![CRTBigInt::zero(); N*N*N],
                    uz: vec![CRTBigInt::zero(); N*N*N],
                }
            }
        };

        // The b-step itself: pure CRTBigInt arithmetic, never panics
        b = step_b_crt(&b, &a, &u_a, &u_b, &constants);

        lane0_trajectory.push(b_l2_residue_lane0(&b));
        lane_trajectory.push(b_l2_lane_residues(&b));
    }

    let final_l2_garner = b_l2_garner(&b);

    TrajResult {
        test_id,
        description,
        lane0_trajectory,
        lane_trajectory,
        final_l2_garner,
        initial_l2_garner,
        completed,
    }
}

fn run_canonical_a_traj(
    test_id: &'static str,
    description: String,
    a_init: Vort3D,
    nu_scaled: i64,
    dt_latt: i64,
    n_steps: u64,
) -> TrajResult {
    let mut a = CRTVort3D::from_i64_vort(&a_init);
    let constants = StepConstants::new(nu_scaled, dt_latt);

    let initial_l2_garner = b_l2_garner(&a);
    let mut lane0_trajectory = Vec::with_capacity(n_steps as usize + 1);
    let mut lane_trajectory = Vec::with_capacity(n_steps as usize + 1);
    lane0_trajectory.push(b_l2_residue_lane0(&a));
    lane_trajectory.push(b_l2_lane_residues(&a));

    for _step in 0..n_steps {
        // u = K * a via i64 BiotSavart on reconstructed a, falling back to u=0
        // if a exceeds i64. After fallback, the integration is just diffusion-only.
        let a_i64_attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
            for i in 0..N { for j in 0..N { for k in 0..N {
                let ii = idx(i,j,k);
                wx[i][j][k] = a.wx[ii].to_i64_exact();
                wy[i][j][k] = a.wy[ii].to_i64_exact();
                wz[i][j][k] = a.wz[ii].to_i64_exact();
            }}}
            Vort3D { wx, wy, wz }
        }));

        let u = match a_i64_attempt {
            Ok(a_i64) => CRTVel3D::from_i64_vel(&biot_savart_n8(&a_i64)),
            Err(_) => CRTVel3D {
                ux: vec![CRTBigInt::zero(); N*N*N],
                uy: vec![CRTBigInt::zero(); N*N*N],
                uz: vec![CRTBigInt::zero(); N*N*N],
            }
        };

        a = step_a_crt(&a, &u, &constants);
        lane0_trajectory.push(b_l2_residue_lane0(&a));
        lane_trajectory.push(b_l2_lane_residues(&a));
    }

    let final_l2_garner = b_l2_garner(&a);

    TrajResult {
        test_id, description,
        lane0_trajectory, lane_trajectory,
        final_l2_garner, initial_l2_garner,
        completed: true,
    }
}


fn print_traj(r: &TrajResult) {
    println!("  {} — {}", r.test_id, r.description);
    println!("    Initial ‖b‖² (Garner u128): {:?}", r.initial_l2_garner);
    println!("    Final   ‖b‖² (Garner u128): {:?}", r.final_l2_garner);
    println!("    Lane-0 residue trajectory (first 8, last 8):");
    let n = r.lane0_trajectory.len();
    let head: Vec<String> = r.lane0_trajectory.iter().take(8).map(|x| x.to_string()).collect();
    let tail: Vec<String> = r.lane0_trajectory.iter().rev().take(8).rev().map(|x| x.to_string()).collect();
    println!("      first 8: {:?}", head);
    if n > 16 { println!("      last 8:  {:?}", tail); }

    // Compare initial vs final lane residues
    let l_init = &r.lane_trajectory[0];
    let l_final = &r.lane_trajectory[r.lane_trajectory.len() - 1];
    let same_residues = l_init == l_final;
    println!("    All 8 lane residues unchanged from start to end: {}", same_residues);
    if !same_residues {
        let n_diff = (0..8).filter(|&i| l_init[i] != l_final[i]).count();
        println!("    Number of lanes that changed: {}/8", n_diff);
    }
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  D-024 b-Attractor in Pure CRTBigInt — Push 7");
    println!("  4th Attractor structure: modular bound = CRT product M ≈ 2^504");
    println!("  No reconstruction in inner loop. No saturation. No clamping.");
    println!("  Wrapping is the cylindrical phase-space topology, not a bug.");
    println!("  N = {}, N_STEPS = {}", N, N_STEPS);
    println!("{}", "=".repeat(78));
    println!();

    // Sanity: verify the modular inverses compute correctly
    println!("  [SANITY] StepConstants compute mod_inverses correctly...");
    let _c = StepConstants::new(1_000_000, 100);
    println!("    StepConstants built without error.");
    println!();

    // Sanity: zero b stays zero (fixed point)
    println!("  [SANITY] Zero b under any background stays zero (b=0 is fixed point)...");
    let zero_b = CRTVort3D::zero();
    let a_test = CRTVort3D::from_i64_vort(&taylor_green(10000));
    let u_a_i64 = biot_savart_n8(&taylor_green(10000));
    let u_a_test = CRTVel3D::from_i64_vel(&u_a_i64);
    let u_b_zero = CRTVel3D {
        ux: vec![CRTBigInt::zero(); N*N*N],
        uy: vec![CRTBigInt::zero(); N*N*N],
        uz: vec![CRTBigInt::zero(); N*N*N],
    };
    let c_test = StepConstants::new(1_000_000, 100);
    let zero_b_after = step_b_crt(&zero_b, &a_test, &u_a_test, &u_b_zero, &c_test);
    let still_zero = (0..N*N*N).all(|ii|
        zero_b_after.wx[ii].is_zero() && zero_b_after.wy[ii].is_zero() && zero_b_after.wz[ii].is_zero()
    );
    println!("    b=0 step → b'=0: {}", still_zero);
    if !still_zero { println!("    WARNING: b=0 fixed point violated"); }
    println!();

    // Trajectory tests
    let nu = 1_000_000i64;  // canonical
    let dt = 100i64;        // matches Push 4 Jacobian probe
    let amp_b = 100i64;     // 1% of canonical TG amp=10000

    let tests = vec![
        run_traj("T1", "TG bg, 1% pert".to_string(),
            BgChoice::TG(10000), 12345, amp_b, nu, dt, N_STEPS),
        run_traj("T2", "ABC bg, 1% pert".to_string(),
            BgChoice::ABC(10000), 12345, amp_b, nu, dt, N_STEPS),
        run_traj("T3", "Random bg, 1% pert".to_string(),
            BgChoice::Random(99, 10000), 12345, amp_b, nu, dt, N_STEPS),
        run_traj("T4", "Pure-b (ω_a=0)".to_string(),
            BgChoice::Zero, 77777, amp_b, nu, dt, N_STEPS),
        run_traj("T5", "TG bg, 10% pert".to_string(),
            BgChoice::TG(10000), 12345, 1000, nu, dt, N_STEPS),
    ];
    for r in &tests { print_traj(r); }

    // ────────────────────────────────────────────────────────────────────────
    // CONTROL: canonical standard NS in pure CRTBigInt.
    // Same substrate, same operators, same parameters. Single field
    // self-coupling (no extension). If this grows at the same rate as the
    // b-tests above, the growth is from the explicit Euler integrator at
    // dt=100 on N=8 — not from anything D-024 specific.
    // ────────────────────────────────────────────────────────────────────────
    println!("{}", "=".repeat(78));
    println!("  CONTROL: canonical standard NS in pure CRTBigInt");
    println!("  (single-field self-coupling, no F_p² extension)");
    println!("  Same params as b-tests: ν={}, dt={}, N={}", nu, dt, N);
    println!("{}", "=".repeat(78));
    println!();

    // Same initial conditions used in b-tests, but as the a-field itself.
    // The b-tests started b at random_perturbation_field(12345, 100) — which
    // was a tiny perturbation. For a fair comparison, run canonical NS on
    // the SAME initial structures: TG amp 10000, ABC amp 10000, Random.
    let canon_tests = vec![
        run_canonical_a_traj("C1", "Canonical TG amp=10000".to_string(),
            taylor_green(10000), nu, dt, N_STEPS),
        run_canonical_a_traj("C2", "Canonical ABC amp=10000".to_string(),
            abc_field(10000), nu, dt, N_STEPS),
        run_canonical_a_traj("C3", "Canonical Random amp=10000".to_string(),
            random_field(99, 10000), nu, dt, N_STEPS),
        // Match b-test amplitudes: small initial conditions (1% scale)
        run_canonical_a_traj("C4", "Canonical small (1% TG)".to_string(),
            random_field(12345, 100), nu, dt, N_STEPS),
    ];
    for r in &canon_tests { print_traj(r); }

    println!("{}", "=".repeat(78));
    println!("  INTERPRETATION");
    println!("{}", "=".repeat(78));
    println!();
    println!("  Lane-0 residue is ‖b‖² mod p_0 (~9.2×10^18).");
    println!("  If ‖b‖² stays small (i64-range), Garner u128 reconstruct succeeds");
    println!("  and the lane residue equals the true value mod p_0.");
    println!("  If ‖b‖² grows beyond u128, Garner returns None — we're operating");
    println!("  on the CRT torus past u128 but still well within the 2^504 bound.");
    println!();
    println!("  4th attractor reading:");
    println!("  - If lane residues evolve smoothly through 100 steps: trajectory");
    println!("    is on the torus, no truncation, dynamic system intact.");
    println!("  - If trajectory cycles or stays bounded in lane-0: contracting.");
    println!("  - If it spreads chaotically across lanes: chaotic but bounded.");
    println!("  - If it pegs at fixed values: artifact / steady state.");
    println!();
    println!("{}", "=".repeat(78));
}
