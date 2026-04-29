//! # D-024 Linearized Jacobian Probe — Power Iteration
//!
//! K_b = 0 makes the b-equation linear in b (a-dependent coefficients only).
//! We don't need to track b magnitudes — we need the spectral radius of the
//! linear operator M(a, t) acting on perturbations.
//!
//! Power iteration: starting from a unit b₀, evolve under the SAME operator
//! the full nonlinear b-step uses. Measure the GROWTH RATIO ‖b(t+1)‖²/‖b(t)‖²
//! per step. If the ratio < 1 consistently, the linearized operator contracts.
//!
//! Why this avoids the truncation problem: we measure RATIOS, not absolute
//! magnitudes. We renormalize b to a fixed reference scale every few steps
//! to keep components in i64 range. The ratio is dimensionless and exact
//! (i128 division of two i128 norms).
//!
//! Outputs:
//!   1. Per-step growth rate λ(t) = log₂(‖b(t+1)‖²/‖b(t)‖²) / 2
//!   2. 30-step composite ratio (the M_30 spectral radius surrogate)
//!   3. Asymptotic Lyapunov estimate λ_max = (1/T) log₂(‖b(T)‖²/‖b(0)‖²) / 2
//!   4. Power iteration: dominant eigenvector of M(a) for fixed a-snapshot

#![forbid(unsafe_code)]

use nsgrid3d::biot_savart_ntt::BiotSavartNTT;
use nsgrid3d::ns_grid_3d::{NSGrid3D, NSCell3D};
use nsgrid3d::fused_solver::{skew_advection_3d, stretching_3d, diffusion_3d};

const N: usize = 8;
const SCALE: i64 = 10_000;
const DT_SCALED: i64 = 100;
const NU_SCALED: i64 = 1_000_000;
const N_STEPS: u64 = 100;
const RENORMALIZE_TARGET: i64 = 1_000_000; // keep ‖b‖_∞ at this scale

// ─── Field initialization ───────────────────────────────────────────────────

fn taylor_green_w(ix: usize, iy: usize, iz: usize, amp: i64) -> [i64; 3] {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let sx = sin_t[ix % 8]; let cx = cos_t[ix % 8];
    let sy = sin_t[iy % 8]; let cy = cos_t[iy % 8];
    let sz = sin_t[iz % 8]; let cz = cos_t[iz % 8];
    let s = amp;
    let w_x = -2 * s * (cx * sy / SCALE) * sz / (SCALE * SCALE);
    let w_y =      s * (sx * cy / SCALE) * sz / (SCALE * SCALE);
    let w_z =      s * (sx * sy / SCALE) * cz / (SCALE * SCALE);
    [w_x, w_y, w_z]
}

fn abc_w(ix: usize, iy: usize, iz: usize, amp: i64) -> [i64; 3] {
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];
    let a = amp; let b = amp; let c = amp;
    let w_x = (a * sin_t[iz % 8] / SCALE) + (c * cos_t[iy % 8] / SCALE);
    let w_y = (b * sin_t[ix % 8] / SCALE) + (a * cos_t[iz % 8] / SCALE);
    let w_z = (c * sin_t[iy % 8] / SCALE) + (b * cos_t[ix % 8] / SCALE);
    [w_x, w_y, w_z]
}

fn random_perturbation(ix: usize, iy: usize, iz: usize, seed: u64, amp: i64) -> [i64; 3] {
    let h = ((ix as u64 * 1009 + iy as u64 * 1013 + iz as u64 * 1019)
             .wrapping_mul(seed.wrapping_add(1))).wrapping_mul(2654435761);
    let raw0 = ((h >>  0) & 0xFFFF) as i64 - 32768;
    let raw1 = ((h >> 16) & 0xFFFF) as i64 - 32768;
    let raw2 = ((h >> 32) & 0xFFFF) as i64 - 32768;
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

// ─── Bilinear cross-coupling (i64 path, same as attractor) ─────────────────

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

// ─── Standard a-step ────────────────────────────────────────────────────────

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

/// Linearized b-step: compute the b-equation explicitly using current
/// a-snapshot. Returns the new b-grid.
///
/// b'(x) = b(x) + dt × [ ν Δb − (u_a · ∇)b − (u_b · ∇)a + (ω_a · ∇)u_b + (ω_b · ∇)u_a ]
///
/// All terms are linear in b (with a-dependent coefficients), so this IS
/// the linearized operator M(a).
fn step_b_linearized(grid_b: &mut NSGrid3D, a_snap: &[NSCell3D]) {
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
                    let diff = diffusion_3d(&b_snap, n, ix, iy, iz, comp, nu);
                    let adv_ab = cross_advection_3d(&b_snap, a_snap, n, ix, iy, iz, comp);
                    let adv_ba = cross_advection_3d(a_snap, &b_snap, n, ix, iy, iz, comp);
                    let str_ab = cross_stretching_3d(a_snap, &b_snap, n, ix, iy, iz, comp);
                    let str_ba = cross_stretching_3d(&b_snap, a_snap, n, ix, iy, iz, comp);
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

// ─── Norms and renormalization ──────────────────────────────────────────────

fn b_l2_sq(grid: &NSGrid3D) -> i128 {
    let mut sum: i128 = 0;
    for cell in &grid.cells {
        sum += (cell.w_x as i128).pow(2)
             + (cell.w_y as i128).pow(2)
             + (cell.w_z as i128).pow(2);
    }
    sum
}

fn b_l_inf(grid: &NSGrid3D) -> i64 {
    grid.cells.iter().map(|c|
        c.w_x.abs().max(c.w_y.abs()).max(c.w_z.abs())
    ).max().unwrap_or(0)
}

/// Rescale b-grid so ‖b‖_∞ = target, return scale factor (numerator/denominator).
/// Returns (numer, denom) such that new_b = old_b × numer / denom.
fn renormalize(grid: &mut NSGrid3D, target: i64) -> (i128, i128) {
    let max = b_l_inf(grid);
    if max == 0 { return (1, 1); }
    // We want max → target, scale factor = target / max.
    // To stay in integers, multiply by target then divide by max.
    for cell in grid.cells.iter_mut() {
        cell.w_x = (cell.w_x as i128 * target as i128 / max as i128) as i64;
        cell.w_y = (cell.w_y as i128 * target as i128 / max as i128) as i64;
        cell.w_z = (cell.w_z as i128 * target as i128 / max as i128) as i64;
    }
    (target as i128, max as i128)
}

// ─── Test runner: power iteration on the linearized b-operator ─────────────

#[derive(Debug, Clone, Copy)]
enum BackgroundField { TG(i64), ABC(i64), Random(i64, u64) }

#[derive(Debug)]
struct JacobianResult {
    test_id: &'static str,
    description: String,
    /// Sequence of per-step growth ratios r(t) = ‖b(t+1)‖/‖b(t)‖ × 1000
    /// (integer, scaled by 1000 for readability)
    growth_ratios_scaled: Vec<i64>,
    /// Final composite growth: total ‖b(T)‖/‖b(0)‖ accumulating renormalizations
    /// log_2 of the total growth × 1000 / T (the asymptotic Lyapunov exponent
    /// per step, scaled)
    lyapunov_log2_x1000: i64,
    verdict: &'static str,
}

fn run_jacobian_probe(
    test_id: &'static str,
    background: BackgroundField,
    seed: u64,
    pure_b_mode: bool,
    nu_override: i64,
    n_steps: u64,
) -> JacobianResult {
    let kernel = BiotSavartNTT::precompute(N);
    let bg_field: Box<dyn Fn(usize, usize, usize) -> [i64; 3]> = match background {
        BackgroundField::TG(amp)        => Box::new(move |x,y,z| taylor_green_w(x, y, z, amp)),
        BackgroundField::ABC(amp)       => Box::new(move |x,y,z| abc_w(x, y, z, amp)),
        BackgroundField::Random(amp, s) => Box::new(move |x,y,z| random_perturbation(x, y, z, s, amp)),
    };
    let mut grid_a = if pure_b_mode {
        build_grid(&|_,_,_| [0i64, 0, 0])
    } else {
        build_grid(&bg_field)
    };
    grid_a.nu_scaled = nu_override;

    // b₀ at fixed renormalized scale
    let mut grid_b = build_grid(&|x,y,z| random_perturbation(x, y, z, seed, RENORMALIZE_TARGET));
    grid_b.nu_scaled = nu_override;

    // Renormalize b₀ to exactly RENORMALIZE_TARGET
    renormalize(&mut grid_b, RENORMALIZE_TARGET);

    let mut growth_ratios = Vec::with_capacity(n_steps as usize);
    // Track total log2 growth across all steps via accumulator.
    // Each renormalization multiplies by (numer/denom); we track sum of
    // log2(numer/denom) as integer-scaled values.
    let mut log2_total_x1000: i64 = 0;

    for step in 0..n_steps {
        // Compute u_a from a-grid
        let u_a = kernel.apply(&grid_a);
        for (i, cell) in grid_a.cells.iter_mut().enumerate() {
            cell.u_x = u_a[i][0]; cell.u_y = u_a[i][1]; cell.u_z = u_a[i][2];
        }
        // Compute u_b from b-grid (uses K_b = 0 fact: u_b = K * b_w componentwise)
        let u_b = kernel.apply(&grid_b);
        for (i, cell) in grid_b.cells.iter_mut().enumerate() {
            cell.u_x = u_b[i][0]; cell.u_y = u_b[i][1]; cell.u_z = u_b[i][2];
        }
        // Snapshot a (with both w and u filled)
        let a_snap = grid_a.cells.clone();

        // Norm before step
        let b_norm_pre = b_l2_sq(&grid_b);

        // Linearized b-step
        step_b_linearized(&mut grid_b, &a_snap);

        // Norm after step
        let b_norm_post = b_l2_sq(&grid_b);

        // Per-step growth ratio (b_norm_post / b_norm_pre, scaled by 1000)
        let ratio_scaled = if b_norm_pre > 0 {
            (b_norm_post * 1000) / b_norm_pre
        } else { 0 };
        growth_ratios.push(ratio_scaled as i64);

        // Renormalize b to keep magnitudes in i64, accumulate log2 of scaling
        let (numer, denom) = renormalize(&mut grid_b, RENORMALIZE_TARGET);
        // log2(denom/numer) * 1000 — we shrunk b, so the b norm before
        // renorm was larger than target. The renormalization scale factor
        // is target/max = numer/denom. b_actual_growth = (renorm_factor)⁻¹.
        // To track growth: log2(max/target) per step.
        // ilog2 trick: 1000 * log2(x) ≈ floor(1000 * (bitlen-1 + frac))
        // Simpler: compute log2(denom) - log2(numer) bit-shift.
        let l_d = ilog2_i128(denom);
        let l_n = ilog2_i128(numer);
        log2_total_x1000 += (l_d - l_n) as i64 * 1000;

        // Advance a-grid
        if !pure_b_mode { step_a(&mut grid_a); }
    }

    // Lyapunov from per-step ratios.
    // Each ratio r_t is ‖b(t+1)‖²/‖b(t)‖² × 1000.
    // The full b² growth ratio is prod(r_t) / 1000^T.
    // log_2(b²(T)/b²(0)) = sum log_2(r_t/1000) = sum (log_2(r_t) - log_2(1000))
    // We want λ = (1/T) × (1/2) × log_2(b²(T)/b²(0)).
    //
    // To get precision below the integer bit, compute:
    //   sum log_2(r_t/1000) ≈ sum (r_t - 1000) / (1000 × ln(2))   (linearization for r ≈ 1000)
    //   ln(2) ≈ 6931 / 10000
    //   So log_2(r/1000) ≈ (r - 1000) × 10000 / (1000 × 6931) = (r - 1000) × 10 / 6931
    //
    // Lyapunov × 1_000_000 (to retain precision):
    //   λ × 1e6 = (1/2) × (1/T) × Σ (r - 1000) × 10 × 1e6 / 6931
    //          = (5e6 / 6931) × Σ(r - 1000) / T
    let n_steps_actual = growth_ratios.len() as i128;
    let sum_dev: i128 = growth_ratios.iter().map(|&r| (r as i128 - 1000)).sum();
    // Lyapunov per step in units of 10^-6 (microbits per step):
    //   λ_per_step × 1e6 = (5_000_000 / 6931) × sum_dev / T
    let lyapunov_per_step_microbit: i64 = if n_steps_actual > 0 {
        ((5_000_000i128 * sum_dev) / (6931 * n_steps_actual)) as i64
    } else { 0 };
    let lyapunov_per_step = lyapunov_per_step_microbit; // log₂/step × 10⁶

    // Verdict from per-step growth ratios
    // ratio < 1000 means ‖b(t+1)‖² < ‖b(t)‖² → contracting
    // ratio > 1000 means expanding
    let n_contracting = growth_ratios.iter().filter(|&&r| r < 1000).count();
    let n_expanding = growth_ratios.iter().filter(|&&r| r > 1000).count();
    let n_neutral = growth_ratios.iter().filter(|&&r| r == 1000).count();

    let verdict = if n_expanding == 0 && n_contracting > 0 {
        "Contracting (all steps decay)"
    } else if n_contracting > n_expanding * 5 {
        "Strongly contracting (avg)"
    } else if n_contracting > n_expanding {
        "Net contracting"
    } else if n_expanding > n_contracting * 5 {
        "Strongly expanding"
    } else if n_expanding > n_contracting {
        "Net expanding"
    } else {
        "Neutral / mixed"
    };
    let _ = n_neutral;

    let description = match background {
        BackgroundField::TG(a)        => format!("TG amp={}", a),
        BackgroundField::ABC(a)       => format!("ABC amp={}", a),
        BackgroundField::Random(a, s) => format!("Random amp={}, seed={}", a, s),
    };
    let description = if pure_b_mode { format!("PURE-B (ω_a=0)") } else { description };

    JacobianResult {
        test_id, description,
        growth_ratios_scaled: growth_ratios,
        lyapunov_log2_x1000: lyapunov_per_step,  // now actually log₂/step × 10⁶
        verdict,
    }
}

fn ilog2_i128(x: i128) -> i32 {
    if x <= 0 { return -1; }
    127 - x.leading_zeros() as i32
}

fn print_result(r: &JacobianResult) {
    println!("  {} — {}", r.test_id, r.description);
    println!("    Lyapunov per step (units of 10⁻⁶ log₂/step): {}",
             r.lyapunov_log2_x1000);
    // Sign reading: negative = b decays, positive = b grows
    let sign = if r.lyapunov_log2_x1000 < 0 { "DECAYS" }
               else if r.lyapunov_log2_x1000 > 0 { "GROWS" }
               else { "neutral (below ~10⁻⁶ resolution)" };
    println!("    → b {}", sign);
    let g = &r.growth_ratios_scaled;
    let n = g.len();
    if n > 0 {
        let mean: i64 = g.iter().sum::<i64>() / n as i64;
        let min = *g.iter().min().unwrap();
        let max = *g.iter().max().unwrap();
        let n_contract = g.iter().filter(|&&x| x < 1000).count();
        let n_expand = g.iter().filter(|&&x| x > 1000).count();
        println!("    Per-step ‖b(t+1)‖²/‖b(t)‖² × 1000:");
        println!("      mean = {}, min = {}, max = {}", mean, min, max);
        println!("      contracting steps = {}/{}, expanding = {}/{}", n_contract, n, n_expand, n);

        // Show first 10 and last 10
        let head: Vec<String> = g.iter().take(10).map(|x| x.to_string()).collect();
        let tail: Vec<String> = g.iter().rev().take(10).rev().map(|x| x.to_string()).collect();
        println!("      first 10: {:?}", head);
        if n > 20 { println!("      last 10:  {:?}", tail); }
    }
    println!("    VERDICT: {}", r.verdict);
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  D-024 LINEARIZED b-JACOBIAN PROBE (Power Iteration)");
    println!("  K_b = 0 ⟹ b-equation is linear in b (a-dependent coefficients).");
    println!("  Measure: per-step ‖b(t+1)‖²/‖b(t)‖² ratio (scale-invariant).");
    println!("  Renormalize b every step to keep magnitudes in i64 range.");
    println!("  N_STEPS = {}, N = {}, ν = {}, dt = {}", N_STEPS, N, NU_SCALED, DT_SCALED);
    println!("{}", "=".repeat(78));
    println!();

    let tests = vec![
        run_jacobian_probe("J1", BackgroundField::TG(10000),  12345, false, NU_SCALED, N_STEPS),
        run_jacobian_probe("J2", BackgroundField::ABC(10000), 12345, false, NU_SCALED, N_STEPS),
        run_jacobian_probe("J3", BackgroundField::TG(100000), 12345, false, NU_SCALED, N_STEPS),
        run_jacobian_probe("J4", BackgroundField::Random(10000, 99), 54321, false, NU_SCALED, N_STEPS),
        run_jacobian_probe("J5", BackgroundField::TG(10000),  77777, true,  NU_SCALED, N_STEPS),
        // Low-ν tests to expose the bilinear coupling alone
        run_jacobian_probe("J6", BackgroundField::TG(10000),  12345, false, NU_SCALED / 100, N_STEPS),
        run_jacobian_probe("J7", BackgroundField::ABC(10000), 12345, false, NU_SCALED / 100, N_STEPS),
    ];

    for r in &tests { print_result(r); }

    println!("{}", "=".repeat(78));
    println!("  INTERPRETATION");
    println!("{}", "=".repeat(78));
    println!();
    println!("  Per-step ratio < 1000  ⟹ b shrinks that step (contracting Jacobian)");
    println!("  Per-step ratio > 1000  ⟹ b grows that step (expanding Jacobian)");
    println!("  Lyapunov < 0           ⟹ asymptotic decay (b → 0 exponentially)");
    println!("  Lyapunov > 0           ⟹ asymptotic growth (b unstable)");
    println!();
    println!("  Pure-b case (J5) is the diffusion control: shows pure ν Δ rate.");
    println!("  J6/J7 (ν/100) isolate the bilinear coupling without diffusion mask.");
    println!();
    println!("  D-024 closure requires Lyapunov < 0 across ALL admissible backgrounds.");
    println!("{}", "=".repeat(78));
}
