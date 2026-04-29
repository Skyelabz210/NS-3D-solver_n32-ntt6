//! # D-024 Attractor Test — b-Component Stability Under NS Evolution
//!
//! K_b = 0 has been confirmed (see d024_probe::main). The b = 0 subspace
//! is invariant under Biot-Savart. The remaining question is whether it
//! is ATTRACTING — does a perturbation ω_b ≠ 0 decay back to zero, or
//! does it grow under the bilinear NS coupling?
//!
//! The exact b-equation (with K_b = 0):
//!
//!   ∂_t b = ν Δb                  (diffusion, stabilizing)
//!         − (u_a · ∇)b              (linear advection by a-velocity)
//!         − (u_b · ∇)a              (cross-advection of a-vorticity by b-velocity)
//!         + (ω_a · ∇)u_b           (stretching of b-velocity by a-vorticity)
//!         + (ω_b · ∇)u_a           (stretching of a-velocity by b-vorticity)
//!
//! Test protocol:
//!   1. Set up (a, b) initial conditions per A1–A6.
//!   2. At each step:
//!      - Compute u_a from ω_a via Biot-Savart (existing kernel)
//!      - Compute u_b from ω_b via Biot-Savart (same kernel, K_b = 0)
//!      - Update ω_a using standard NS step (existing path)
//!      - Update ω_b using the cross-coupled b-equation above
//!   3. Track ‖b(t)‖₂, ‖a(t)‖₂, λ_eff(t), R_stretch(t).
//!   4. Classify: Attracting / Neutral / Unstable.
//!
//! All arithmetic exact integer. Zero float.

#![forbid(unsafe_code)]

use nsgrid3d::biot_savart_ntt::BiotSavartNTT;
use nsgrid3d::ns_grid_3d::{NSGrid3D, NSCell3D};
use nsgrid3d::fused_solver::{skew_advection_3d, stretching_3d, diffusion_3d};

const N: usize = 8;
const SCALE: i64 = 10_000;
const DT_SCALED: i64 = 100;       // dt = 0.01 (canonical from solver default)
const NU_SCALED: i64 = 1_000_000; // ν = 0.01 (Re = 100)
const N_STEPS: u64 = 100;

// ─── Field initialization ───────────────────────────────────────────────────

/// Taylor-Green vorticity at scaled SCALE.
fn taylor_green_w(ix: usize, iy: usize, iz: usize, amp: i64) -> [i64; 3] {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let sx = sin_t[ix % 8]; let cx = cos_t[ix % 8];
    let sy = sin_t[iy % 8]; let cy = cos_t[iy % 8];
    let sz = sin_t[iz % 8]; let cz = cos_t[iz % 8];
    // amp scales the magnitude (in SCALE units)
    let s = amp;
    let w_x = -2 * s * (cx * sy / SCALE) * sz / (SCALE * SCALE);
    let w_y =      s * (sx * cy / SCALE) * sz / (SCALE * SCALE);
    let w_z =      s * (sx * sy / SCALE) * cz / (SCALE * SCALE);
    [w_x, w_y, w_z]
}

/// ABC (Beltrami) vorticity: u = ω, perfect alignment, strongest stretching.
fn abc_w(ix: usize, iy: usize, iz: usize, amp: i64) -> [i64; 3] {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let A = amp; let B = amp; let C = amp;
    let w_x = (A * sin_t[iz % 8] / SCALE) + (C * cos_t[iy % 8] / SCALE);
    let w_y = (B * sin_t[ix % 8] / SCALE) + (A * cos_t[iz % 8] / SCALE);
    let w_z = (C * sin_t[iy % 8] / SCALE) + (B * cos_t[ix % 8] / SCALE);
    [w_x, w_y, w_z]
}

/// Pseudorandom divergence-free perturbation, deterministic from seed.
/// Generates values in [-amp, amp] scaled by SCALE.
fn random_perturbation(ix: usize, iy: usize, iz: usize, seed: u64, amp: i64) -> [i64; 3] {
    let h = ((ix as u64 * 1009 + iy as u64 * 1013 + iz as u64 * 1019)
             .wrapping_mul(seed.wrapping_add(1))).wrapping_mul(2654435761);
    // Each component: hash bits → [-32768, 32767] → scale to [-amp, amp]
    let raw0 = ((h >> 0)  & 0xFFFF) as i64 - 32768;
    let raw1 = ((h >> 16) & 0xFFFF) as i64 - 32768;
    let raw2 = ((h >> 32) & 0xFFFF) as i64 - 32768;
    // amp is in SCALE units. Multiply first, divide second to preserve precision.
    let v0 = (raw0 as i128 * amp as i128 / 32768) as i64;
    let v1 = (raw1 as i128 * amp as i128 / 32768) as i64;
    let v2 = (raw2 as i128 * amp as i128 / 32768) as i64;
    [v0, v1, v2]
}

fn build_grid(field: &dyn Fn(usize, usize, usize) -> [i64; 3]) -> NSGrid3D {
    let mut g = NSGrid3D::new(N, 5, DT_SCALED);
    g.nu_scaled = NU_SCALED;
    for ix in 0..N {
        for iy in 0..N {
            for iz in 0..N {
                let w = field(ix, iy, iz);
                let idx = ix * N * N + iy * N + iz;
                g.cells[idx] = NSCell3D {
                    w_x: w[0], w_y: w[1], w_z: w[2],
                    u_x: 0, u_y: 0, u_z: 0,
                };
            }
        }
    }
    g
}

// ─── Bilinear cross-coupling operators ──────────────────────────────────────

/// Cross-advection (u_src · ∇)ω_src at a point.
/// u_src provides velocity, ω_src provides vorticity-component.
/// Returns 2h · (-(u · ∇)ω_comp) — same sign convention as skew_advection_3d.
fn cross_advection_3d(
    omega_snap: &[NSCell3D],
    u_snap: &[NSCell3D],
    n: usize,
    ix: usize, iy: usize, iz: usize,
    comp: usize,
) -> i64 {
    let idx = |x: usize, y: usize, z: usize| -> usize { x * n * n + y * n + z };
    let wrap = |i: usize, delta: isize| -> usize {
        ((i as isize + delta).rem_euclid(n as isize)) as usize
    };
    let get_w = |cell: &NSCell3D| -> i64 {
        match comp { 0 => cell.w_x, 1 => cell.w_y, _ => cell.w_z }
    };
    let get_u = |cell: &NSCell3D, d: usize| -> i64 {
        match d { 0 => cell.u_x, 1 => cell.u_y, _ => cell.u_z }
    };

    let neighbors = [
        (wrap(ix,1), iy, iz, wrap(ix,-1), iy, iz),
        (ix, wrap(iy,1), iz, ix, wrap(iy,-1), iz),
        (ix, iy, wrap(iz,1), ix, iy, wrap(iz,-1)),
    ];

    let mut result: i64 = 0;
    for (d, &(xp, yp, zp, xm, ym, zm)) in neighbors.iter().enumerate() {
        let u_d_at_c = get_u(&u_snap[idx(ix, iy, iz)], d);
        let w_p = get_w(&omega_snap[idx(xp, yp, zp)]);
        let w_m = get_w(&omega_snap[idx(xm, ym, zm)]);
        let u_p = get_u(&u_snap[idx(xp, yp, zp)], d);
        let u_m = get_u(&u_snap[idx(xm, ym, zm)], d);

        let standard = (u_d_at_c as i128 * (w_p - w_m) as i128) / SCALE as i128;
        let div_form = ((u_p as i128 * w_p as i128) - (u_m as i128 * w_m as i128))
                       / SCALE as i128;
        result += ((standard + div_form) / 2) as i64;
    }
    -result
}

/// Cross-stretching (ω_src · ∇)u_src at a point.
/// Returns 2h · ((ω · ∇)u_comp).
fn cross_stretching_3d(
    omega_snap: &[NSCell3D],
    u_snap: &[NSCell3D],
    n: usize,
    ix: usize, iy: usize, iz: usize,
    comp: usize,
) -> i64 {
    let idx = |x: usize, y: usize, z: usize| -> usize { x * n * n + y * n + z };
    let wrap = |i: usize, delta: isize| -> usize {
        ((i as isize + delta).rem_euclid(n as isize)) as usize
    };
    let get_u = |cell: &NSCell3D, d: usize| -> i64 {
        match d { 0 => cell.u_x, 1 => cell.u_y, _ => cell.u_z }
    };

    let c_omega = &omega_snap[idx(ix, iy, iz)];
    let neighbors = [
        (wrap(ix,1), iy, iz, wrap(ix,-1), iy, iz),
        (ix, wrap(iy,1), iz, ix, wrap(iy,-1), iz),
        (ix, iy, wrap(iz,1), ix, iy, wrap(iz,-1)),
    ];

    let mut result: i64 = 0;
    for (d, &(xp, yp, zp, xm, ym, zm)) in neighbors.iter().enumerate() {
        let u_p = get_u(&u_snap[idx(xp, yp, zp)], comp);
        let u_m = get_u(&u_snap[idx(xm, ym, zm)], comp);
        let w_d = match d { 0 => c_omega.w_x, 1 => c_omega.w_y, _ => c_omega.w_z };
        result += ((w_d as i128 * (u_p - u_m) as i128) / SCALE as i128) as i64;
    }
    result
}

// ─── Step functions ─────────────────────────────────────────────────────────

/// Update the a-grid using the existing standard NS step (advection + stretching + diffusion).
/// u_a must be filled before calling.
fn step_a(grid_a: &mut NSGrid3D) {
    let n = grid_a.n;
    let snap = grid_a.cells.clone();
    let nu = grid_a.nu_scaled;
    let dt = grid_a.dt_scaled;
    for ix in 0..n {
        for iy in 0..n {
            for iz in 0..n {
                let idx = ix * n * n + iy * n + iz;
                let mut new_w = [0i64; 3];
                for comp in 0..3 {
                    let adv = skew_advection_3d(&snap, n, ix, iy, iz, comp);
                    let str_ = stretching_3d(&snap, n, ix, iy, iz, comp);
                    let diff = diffusion_3d(&snap, n, ix, iy, iz, comp, nu);
                    // dt × (advection + stretching + diffusion)
                    let dwdt = adv as i128 + str_ as i128 + diff as i128;
                    let delta = (dt as i128 * dwdt) / SCALE as i128;
                    let w_old = match comp { 0 => snap[idx].w_x, 1 => snap[idx].w_y, _ => snap[idx].w_z };
                    new_w[comp] = w_old + delta as i64;
                }
                grid_a.cells[idx].w_x = new_w[0];
                grid_a.cells[idx].w_y = new_w[1];
                grid_a.cells[idx].w_z = new_w[2];
            }
        }
    }
}

/// Update the b-grid using the cross-coupled b-equation:
///   ∂_t b = ν Δb − (u_a · ∇)b − (u_b · ∇)a + (ω_a · ∇)u_b + (ω_b · ∇)u_a
fn step_b(grid_b: &mut NSGrid3D, a_snap: &[NSCell3D]) {
    let n = grid_b.n;
    let b_snap = grid_b.cells.clone();
    let nu = grid_b.nu_scaled;
    let dt = grid_b.dt_scaled;
    for ix in 0..n {
        for iy in 0..n {
            for iz in 0..n {
                let idx = ix * n * n + iy * n + iz;
                let mut new_w = [0i64; 3];
                for comp in 0..3 {
                    // ν Δb on b-snapshot
                    let diff = diffusion_3d(&b_snap, n, ix, iy, iz, comp, nu);

                    // -(u_a · ∇)b: u from a-snap, ω from b-snap
                    // skew_advection signature is "advection of ω given u", here we reuse
                    // by building a temporary snapshot that has b's vorticity and a's velocity.
                    // cross_advection_3d does exactly this directly.
                    let adv_ab = cross_advection_3d(&b_snap, a_snap, n, ix, iy, iz, comp);

                    // -(u_b · ∇)a: u from b-snap, ω from a-snap
                    let adv_ba = cross_advection_3d(a_snap, &b_snap, n, ix, iy, iz, comp);

                    // (ω_a · ∇)u_b: ω from a-snap, u from b-snap
                    let str_ab = cross_stretching_3d(a_snap, &b_snap, n, ix, iy, iz, comp);

                    // (ω_b · ∇)u_a: ω from b-snap, u from a-snap
                    let str_ba = cross_stretching_3d(&b_snap, a_snap, n, ix, iy, iz, comp);

                    // db/dt = ν Δb + adv_ab + adv_ba + str_ab + str_ba
                    // Note adv_ab and adv_ba are already negated (cross_advection returns -result)
                    let dwdt = diff as i128 + adv_ab as i128 + adv_ba as i128
                             + str_ab as i128 + str_ba as i128;
                    let delta = (dt as i128 * dwdt) / SCALE as i128;
                    let w_old = match comp { 0 => b_snap[idx].w_x, 1 => b_snap[idx].w_y, _ => b_snap[idx].w_z };
                    new_w[comp] = w_old + delta as i64;
                }
                grid_b.cells[idx].w_x = new_w[0];
                grid_b.cells[idx].w_y = new_w[1];
                grid_b.cells[idx].w_z = new_w[2];
            }
        }
    }
}

// ─── Measurements ───────────────────────────────────────────────────────────

/// L² norm² of vorticity field, in (SCALE units)² — exact i128.
/// Returns Σ (w_x² + w_y² + w_z²).
fn l2_sq(grid: &NSGrid3D) -> i128 {
    let mut sum: i128 = 0;
    for cell in &grid.cells {
        sum += (cell.w_x as i128).pow(2)
             + (cell.w_y as i128).pow(2)
             + (cell.w_z as i128).pow(2);
    }
    sum
}

/// Approximate integer log₂ of an i128, returns -1 if zero.
fn ilog2_i128(x: i128) -> i32 {
    if x <= 0 { return -1; }
    127 - x.leading_zeros() as i32
}

/// Effective growth rate λ_eff(t) = (1/t) log(‖b(t)‖₂ / ‖b(0)‖₂)
/// Reported as integer log₂ ratio scaled by 1000/t for human readability.
/// Returns scaled value: 1000 × log₂(ratio) / t.
/// Positive = growth, negative = decay, zero = neutral.
fn lambda_eff_scaled(b_t_sq: i128, b_0_sq: i128, t: u64) -> i64 {
    if b_0_sq == 0 || t == 0 { return 0; }
    // log₂(‖b(t)‖² / ‖b(0)‖²) = log₂(b_t_sq) - log₂(b_0_sq)
    // Then divide by 2 (norm vs norm²) and by t.
    let lt = ilog2_i128(b_t_sq);
    let l0 = ilog2_i128(b_0_sq);
    let ratio_log2_x1000 = ((lt as i64 - l0 as i64) * 1000) / 2;
    ratio_log2_x1000 / t as i64
}

// ─── Test cases ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum BackgroundField { TG(i64), ABC(i64), Random(i64, u64) }

#[derive(Debug)]
struct AttractorResult {
    test_id: &'static str,
    description: String,
    a_l2_sq_initial: i128,
    b_l2_sq_initial: i128,
    a_l2_sq_final: i128,
    b_l2_sq_final: i128,
    b_l2_sq_max: i128,
    b_l2_sq_min: i128,
    lambda_eff: i64,            // scaled by 1000/t
    verdict: &'static str,
}

fn run_attractor_test(
    test_id: &'static str,
    background: BackgroundField,
    perturb_pct: i64,            // perturbation as % of background magnitude
    seed: u64,
    pure_b_mode: bool,           // if true, ω_a = 0
) -> AttractorResult {
    run_attractor_test_with_nu(test_id, background, perturb_pct, seed, pure_b_mode, NU_SCALED)
}

fn run_attractor_test_with_nu(
    test_id: &'static str,
    background: BackgroundField,
    perturb_pct: i64,
    seed: u64,
    pure_b_mode: bool,
    nu_override: i64,
) -> AttractorResult {
    let kernel = BiotSavartNTT::precompute(N);

    // Build a-field
    let bg_field: Box<dyn Fn(usize, usize, usize) -> [i64; 3]> = match background {
        BackgroundField::TG(amp)    => Box::new(move |x,y,z| taylor_green_w(x, y, z, amp)),
        BackgroundField::ABC(amp)   => Box::new(move |x,y,z| abc_w(x, y, z, amp)),
        BackgroundField::Random(amp, s) => Box::new(move |x,y,z| random_perturbation(x, y, z, s, amp)),
    };

    let mut grid_a = if pure_b_mode {
        // ω_a = 0 case
        build_grid(&|_,_,_| [0i64, 0, 0])
    } else {
        build_grid(&bg_field)
    };
    grid_a.nu_scaled = nu_override;

    // Build b-perturbation: scaled fraction of background magnitude
    let bg_amp = match background {
        BackgroundField::TG(a) | BackgroundField::ABC(a) | BackgroundField::Random(a, _) => a,
    };
    let pert_amp = (bg_amp * perturb_pct) / 100;
    let mut grid_b = build_grid(&|x, y, z| random_perturbation(x, y, z, seed, pert_amp));
    grid_b.nu_scaled = nu_override;

    let a_l2_initial = l2_sq(&grid_a);
    let b_l2_initial = l2_sq(&grid_b);

    let mut b_l2_max = b_l2_initial;
    let mut b_l2_min = b_l2_initial;

    // Evolve N_STEPS coupled NS steps
    for _step in 0..N_STEPS {
        // 1. Compute u_a from ω_a via Biot-Savart, stash into grid_a
        let u_a = kernel.apply(&grid_a);
        for (i, cell) in grid_a.cells.iter_mut().enumerate() {
            cell.u_x = u_a[i][0];
            cell.u_y = u_a[i][1];
            cell.u_z = u_a[i][2];
        }
        // 2. Compute u_b from ω_b via Biot-Savart, stash into grid_b
        let u_b = kernel.apply(&grid_b);
        for (i, cell) in grid_b.cells.iter_mut().enumerate() {
            cell.u_x = u_b[i][0];
            cell.u_y = u_b[i][1];
            cell.u_z = u_b[i][2];
        }
        // 3. Snapshot a's full state (w + u) for use in step_b
        let a_snap = grid_a.cells.clone();
        // 4. Update both grids
        if !pure_b_mode { step_a(&mut grid_a); }
        step_b(&mut grid_b, &a_snap);

        // Track extremes
        let bsq = l2_sq(&grid_b);
        if bsq > b_l2_max { b_l2_max = bsq; }
        if bsq < b_l2_min { b_l2_min = bsq; }
    }

    let a_l2_final = l2_sq(&grid_a);
    let b_l2_final = l2_sq(&grid_b);
    let lambda = lambda_eff_scaled(b_l2_final, b_l2_initial, N_STEPS);

    // Verdict logic
    // Compare b_l2_final vs initial
    let verdict = if b_l2_final == 0 {
        "Attracting (exact zero)"
    } else if b_l2_final < b_l2_initial / 2 {
        "Attracting"
    } else if b_l2_final < b_l2_initial * 2 {
        "Bounded"
    } else if b_l2_final < b_l2_initial * 100 {
        "Growing (linear)"
    } else {
        "Growing (exponential)"
    };

    let description = match background {
        BackgroundField::TG(a)        => format!("TG amp={}, pert={}%", a, perturb_pct),
        BackgroundField::ABC(a)       => format!("ABC amp={}, pert={}%", a, perturb_pct),
        BackgroundField::Random(a, s) => format!("Random amp={}, seed={}, pert={}%", a, s, perturb_pct),
    };
    let description = if pure_b_mode {
        format!("PURE-B (ω_a=0), {}", description)
    } else {
        description
    };

    AttractorResult {
        test_id,
        description,
        a_l2_sq_initial: a_l2_initial,
        b_l2_sq_initial: b_l2_initial,
        a_l2_sq_final: a_l2_final,
        b_l2_sq_final: b_l2_final,
        b_l2_sq_max: b_l2_max,
        b_l2_sq_min: b_l2_min,
        lambda_eff: lambda,
        verdict,
    }
}

fn print_result(r: &AttractorResult) {
    println!("  {} — {}", r.test_id, r.description);
    println!("    ‖a‖₂² initial: {:>20}", r.a_l2_sq_initial);
    println!("    ‖a‖₂² final:   {:>20}", r.a_l2_sq_final);
    println!("    ‖b‖₂² initial: {:>20}", r.b_l2_sq_initial);
    println!("    ‖b‖₂² final:   {:>20}", r.b_l2_sq_final);
    println!("    ‖b‖₂² max:     {:>20}", r.b_l2_sq_max);
    println!("    ‖b‖₂² min:     {:>20}", r.b_l2_sq_min);
    if r.b_l2_sq_initial > 0 {
        // Ratio b_final² / b_initial² × 1000 for readable percentage
        let ratio_x1000 = (r.b_l2_sq_final * 1000) / r.b_l2_sq_initial.max(1);
        println!("    ratio b²(T)/b²(0): {}/1000", ratio_x1000);
    }
    println!("    λ_eff (×1000/t):  {}", r.lambda_eff);
    println!("    VERDICT: {}", r.verdict);
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  D-024 ATTRACTOR TEST");
    println!("  Question: Does perturbation ω_b ≠ 0 decay under NS evolution?");
    println!("  Substrate: K_b = 0 confirmed, b-subspace invariant.");
    println!("  N_STEPS = {}, N = {}, ν_scaled = {}, dt_scaled = {}", N_STEPS, N, NU_SCALED, DT_SCALED);
    println!("{}", "=".repeat(78));
    println!();

    let tests = vec![
        run_attractor_test("A1", BackgroundField::TG(10000),  1, 12345, false),
        run_attractor_test("A2", BackgroundField::TG(10000), 10, 12345, false),
        run_attractor_test("A3", BackgroundField::TG(100000), 1, 12345, false),
        run_attractor_test("A4", BackgroundField::ABC(10000), 1, 12345, false),
        run_attractor_test("A5", BackgroundField::Random(10000, 99), 1, 54321, false),
        run_attractor_test("A6", BackgroundField::TG(10000),  1, 77777, true),
    ];

    for r in &tests { print_result(r); }

    // Cross-comparison to isolate bilinear effect vs diffusion alone
    println!("{}", "=".repeat(78));
    println!("  CONTROL: Decay ratio comparison (full coupling vs pure-b, same b₀)");
    println!("{}", "=".repeat(78));
    println!();

    // Run A1's exact b₀ with TG background AND with ω_a=0, compare ratios
    // A1: TG amp=10000, b-pert=1%, seed=12345, with full coupling
    // C1: same b₀ but with ω_a=0 (only diffusion + b-self-coupling)
    let a1_full = run_attractor_test("A1-full", BackgroundField::TG(10000), 1, 12345, false);
    let a1_pure = run_attractor_test("A1-pure", BackgroundField::TG(10000), 1, 12345, true);
    println!("  TG bg, b₀ from seed 12345:");
    println!("    Full coupling:  ‖b(T)‖²/‖b(0)‖² = {}/1000  (ω_a active)",
             (a1_full.b_l2_sq_final * 1000) / a1_full.b_l2_sq_initial.max(1));
    println!("    Pure diffusion: ‖b(T)‖²/‖b(0)‖² = {}/1000  (ω_a = 0)",
             (a1_pure.b_l2_sq_final * 1000) / a1_pure.b_l2_sq_initial.max(1));
    let coupling_effect_a1 = a1_full.b_l2_sq_final * 1000 / a1_pure.b_l2_sq_final.max(1);
    println!("    Coupling/Diffusion ratio: {}/1000", coupling_effect_a1);
    if coupling_effect_a1 < 1000 {
        println!("    → Coupling ACCELERATES decay (bilinear terms are dissipative)");
    } else if coupling_effect_a1 > 1000 {
        println!("    → Coupling SLOWS decay (bilinear terms are productive)");
    } else {
        println!("    → Coupling NEUTRAL");
    }
    println!();

    let a4_full = run_attractor_test("A4-full", BackgroundField::ABC(10000), 1, 12345, false);
    let a4_pure = run_attractor_test("A4-pure", BackgroundField::ABC(10000), 1, 12345, true);
    println!("  ABC bg (Beltrami, strongest coupling), b₀ from seed 12345:");
    println!("    Full coupling:  ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (a4_full.b_l2_sq_final * 1000) / a4_full.b_l2_sq_initial.max(1));
    println!("    Pure diffusion: ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (a4_pure.b_l2_sq_final * 1000) / a4_pure.b_l2_sq_initial.max(1));
    let coupling_effect_a4 = a4_full.b_l2_sq_final * 1000 / a4_pure.b_l2_sq_final.max(1);
    println!("    Coupling/Diffusion ratio: {}/1000", coupling_effect_a4);
    if coupling_effect_a4 < 1000 {
        println!("    → Coupling ACCELERATES decay");
    } else if coupling_effect_a4 > 1000 {
        println!("    → Coupling SLOWS decay (productive — danger sign)");
    } else {
        println!("    → Coupling NEUTRAL");
    }
    println!();

    // Low-viscosity diagnostic: run with NU/100 to isolate bilinear coupling
    println!("{}", "=".repeat(78));
    println!("  LOW-NU DIAGNOSTIC: bilinear coupling without diffusion masking");
    println!("{}", "=".repeat(78));
    println!();

    // Re-run the most stringent tests with much smaller ν.
    // We do this by mutating the test grids' nu_scaled.
    // (Refactor: copy run_attractor_test inline with nu override.)
    println!("  Running A2-style (TG bg, 10% pert) with ν reduced 100×...");
    let r_low = run_attractor_test_with_nu("A2-lowν", BackgroundField::TG(10000), 10, 12345, false, NU_SCALED / 100);
    let r_low_pure = run_attractor_test_with_nu("A2-lowν-pure", BackgroundField::TG(10000), 10, 12345, true, NU_SCALED / 100);
    println!("    Full coupling, ν/100:  ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (r_low.b_l2_sq_final * 1000) / r_low.b_l2_sq_initial.max(1));
    println!("    Pure diff, ν/100:      ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (r_low_pure.b_l2_sq_final * 1000) / r_low_pure.b_l2_sq_initial.max(1));
    if r_low.b_l2_sq_final < r_low_pure.b_l2_sq_final {
        println!("    → Bilinear coupling ACCELERATES decay even at low ν");
    } else if r_low.b_l2_sq_final > r_low_pure.b_l2_sq_final {
        println!("    → Bilinear coupling PRODUCES b at low ν (danger sign)");
    } else {
        println!("    → Coupling effect below quantization at low ν");
    }
    println!();

    // ABC at low ν - the most dangerous combination
    println!("  Running A4-style (ABC, 10% pert) with ν reduced 100×...");
    let r_abc = run_attractor_test_with_nu("A4-lowν", BackgroundField::ABC(10000), 10, 12345, false, NU_SCALED / 100);
    let r_abc_pure = run_attractor_test_with_nu("A4-lowν-pure", BackgroundField::ABC(10000), 10, 12345, true, NU_SCALED / 100);
    println!("    Full coupling, ν/100:  ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (r_abc.b_l2_sq_final * 1000) / r_abc.b_l2_sq_initial.max(1));
    println!("    Pure diff, ν/100:      ‖b(T)‖²/‖b(0)‖² = {}/1000",
             (r_abc_pure.b_l2_sq_final * 1000) / r_abc_pure.b_l2_sq_initial.max(1));
    if r_abc.b_l2_sq_final < r_abc_pure.b_l2_sq_final {
        println!("    → ABC bilinear coupling DISSIPATIVE at low ν");
    } else if r_abc.b_l2_sq_final > r_abc_pure.b_l2_sq_final {
        println!("    → ABC bilinear coupling PRODUCTIVE at low ν (danger sign — Beltrami flow drives b growth)");
    } else {
        println!("    → ABC coupling effect below quantization");
    }
    println!();

    // Summary
    println!("{}", "=".repeat(78));
    println!("  SUMMARY");
    println!("{}", "=".repeat(78));
    let attracting = tests.iter().filter(|r| r.verdict.starts_with("Attracting")).count();
    let bounded = tests.iter().filter(|r| r.verdict == "Bounded").count();
    let growing = tests.iter().filter(|r| r.verdict.contains("Growing")).count();
    println!("  Attracting: {}", attracting);
    println!("  Bounded:    {}", bounded);
    println!("  Growing:    {}", growing);
    println!();

    if attracting == tests.len() {
        println!("  D-024 EVIDENCE: b = 0 is an attractor across all tests.");
        println!("    The bilinear NS coupling is dissipative for the b-component.");
        println!("    Lean 4 target: prove ρ(M_30) < 1 for the linearized b-operator.");
    } else if growing > 0 {
        println!("  D-024 COUNTEREXAMPLE: b grows in {} test(s).", growing);
        println!("    The b-subspace is not attracting under bilinear coupling.");
        println!("    D-024 must be reformulated as a bound on ‖b‖, not b = 0.");
    } else {
        println!("  D-024 STATUS MIXED: bounded but not attracting in some cases.");
        println!("    The b-component is conserved or marginally stable.");
        println!("    Investigate: is there a Casimir invariant?");
    }
    println!("{}", "=".repeat(78));
}
