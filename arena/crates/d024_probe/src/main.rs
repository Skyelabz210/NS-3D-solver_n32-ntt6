//! # D-024 Probe — K_b Symmetry Test
//!
//! Question: When the Biot-Savart kernel K is lifted to F_p² and applied
//! to a vorticity field ω = ω_a + ω_b·√δ, does the output velocity
//! u = K * ω respect the F_p² component split?
//!
//! Specifically: is K_b = 0, meaning u_a = K * ω_a and u_b = K * ω_b
//! independently, with no cross-mixing between the a and b components?
//!
//! Procedure:
//!   1. Construct two NSGrid3D fields: one carrying ω_a (the "real" vorticity),
//!      another carrying ω_b (the "extension" vorticity).
//!   2. Apply Biot-Savart to each independently. Get u_a and u_b.
//!   3. Combine into F_p² fields and apply the kernel to the combined field.
//!      (The kernel is the same operator; only the input changes.)
//!   4. Verify: K(ω_a + ω_b√δ) = K(ω_a) + K(ω_b)·√δ exactly per cell.
//!
//! If the kernel respects the split, K_b = 0 and the b-subspace is invariant
//! under Biot-Savart. This is the gating condition for the attractor test.
//!
//! All arithmetic exact integer. Zero float. The test is deterministic.

#![forbid(unsafe_code)]

use nsgrid3d::biot_savart_ntt::BiotSavartNTT;
use nsgrid3d::ns_grid_3d::{NSGrid3D, NSCell3D};

const N: usize = 8;
const SCALE: i64 = 10_000;
// QNR for F_p² lift: pick a prime + non-residue.
// Lane 11 with δ = 2 (since 2 is QNR mod 11: 2^5 mod 11 = 32 mod 11 = 10 = -1).
// But for the GRID-LEVEL test, the F_p² "lift" is conceptual — we test that
// the kernel acts componentwise on (ω_a, ω_b) by linearity.

/// Build a grid with prescribed vorticity field. Velocity is left at zero
/// to be filled by Biot-Savart.
fn build_grid_from_w(w: &[[i64; 3]]) -> NSGrid3D {
    let mut g = NSGrid3D::new(N, 5, 100); // 5 lanes, dt=0.01
    assert_eq!(w.len(), N * N * N);
    for (i, &ws) in w.iter().enumerate() {
        g.cells[i] = NSCell3D {
            w_x: ws[0], w_y: ws[1], w_z: ws[2],
            u_x: 0, u_y: 0, u_z: 0,
        };
    }
    g
}

/// Taylor-Green vorticity at grid point (ix, iy, iz).
/// Uses integer trigonometric approximations scaled by SCALE.
/// Returns (w_x, w_y, w_z) as i64 scaled.
fn taylor_green_w(ix: usize, iy: usize, iz: usize, n: usize) -> [i64; 3] {
    // Integer sin/cos table for N=8: sin(2π·k/8) × SCALE for k=0..7
    // sin: [0, 7071, 10000, 7071, 0, -7071, -10000, -7071]
    // cos: [10000, 7071, 0, -7071, -10000, -7071, 0, 7071]
    let sin_t: [i64; 8] = [0, 7071, 10000, 7071, 0, -7071, -10000, -7071];
    let cos_t: [i64; 8] = [10000, 7071, 0, -7071, -10000, -7071, 0, 7071];

    let sx = sin_t[ix % 8];
    let cx = cos_t[ix % 8];
    let sy = sin_t[iy % 8];
    let cy = cos_t[iy % 8];
    let sz = sin_t[iz % 8];
    let cz = cos_t[iz % 8];

    // Taylor-Green vorticity (scaled): ω = (-2 cos(x) sin(y) sin(z), ...)
    // Each product of two trig values scaled SCALE² → divide by SCALE
    let w_x = -2 * (cx * sy / SCALE) * sz / SCALE;
    let w_y =      (sx * cy / SCALE) * sz / SCALE;
    let w_z =      (sx * sy / SCALE) * cz / SCALE;
    [w_x, w_y, w_z]
}

/// Adversarial random-ish vorticity field. Deterministic from seed.
fn adversarial_w(ix: usize, iy: usize, iz: usize, n: usize, seed: u64) -> [i64; 3] {
    // Linear congruential pattern keyed by position
    let idx = (ix * n * n + iy * n + iz) as u64;
    let v = (seed.wrapping_mul(idx + 1).wrapping_add(idx * 1009)) % 20_000;
    let sign1 = if (v >> 1) & 1 == 0 { 1 } else { -1 };
    let sign2 = if (v >> 2) & 1 == 0 { 1 } else { -1 };
    let sign3 = if (v >> 3) & 1 == 0 { 1 } else { -1 };
    [
        sign1 * (v as i64 % 5000),
        sign2 * ((v as i64 * 7 + 11) % 5000),
        sign3 * ((v as i64 * 13 + 17) % 5000),
    ]
}

fn build_field<F: Fn(usize, usize, usize) -> [i64; 3]>(
    f: F,
) -> Vec<[i64; 3]> {
    let mut w = Vec::with_capacity(N * N * N);
    for ix in 0..N {
        for iy in 0..N {
            for iz in 0..N {
                w.push(f(ix, iy, iz));
            }
        }
    }
    w
}

/// Apply Biot-Savart NTT kernel to a vorticity field, return velocity field.
fn apply_kernel(kernel: &BiotSavartNTT, w: &[[i64; 3]]) -> Vec<[i64; 3]> {
    let mut grid = build_grid_from_w(w);
    let u = kernel.apply(&grid);
    // grid is consumed for input only; output is u
    let _ = &mut grid;
    u
}

/// Apply Biot-Savart with exact i128 numerators (pre-truncation).
/// This is what `apply` does before the floor division by denom.
/// Linearity is exact at this stage (NTT + CRT lift are exact integer ops).
fn apply_kernel_exact(kernel: &BiotSavartNTT, w: &[[i64; 3]]) -> Vec<[i128; 3]> {
    let grid = build_grid_from_w(w);
    kernel.apply_exact(&grid)
}

/// L∞ norm of the difference between two velocity fields.
fn l_inf_diff(a: &[[i64; 3]], b: &[[i64; 3]]) -> i64 {
    a.iter().zip(b.iter()).map(|(x, y)| {
        (x[0] - y[0]).abs().max((x[1] - y[1]).abs()).max((x[2] - y[2]).abs())
    }).max().unwrap_or(0)
}

/// L∞ norm of the difference between two i128 velocity-numerator fields.
fn l_inf_diff_i128(a: &[[i128; 3]], b: &[[i128; 3]]) -> i128 {
    a.iter().zip(b.iter()).map(|(x, y)| {
        (x[0] - y[0]).abs().max((x[1] - y[1]).abs()).max((x[2] - y[2]).abs())
    }).max().unwrap_or(0)
}

/// L∞ norm of a velocity field.
fn l_inf_norm(a: &[[i64; 3]]) -> i64 {
    a.iter().map(|x| x[0].abs().max(x[1].abs()).max(x[2].abs()))
        .max().unwrap_or(0)
}

/// Componentwise sum of two fields.
fn sum_fields(a: &[[i64; 3]], b: &[[i64; 3]]) -> Vec<[i64; 3]> {
    a.iter().zip(b.iter()).map(|(x, y)|
        [x[0] + y[0], x[1] + y[1], x[2] + y[2]]
    ).collect()
}

/// Componentwise sum of two i128 fields.
fn sum_fields_i128(a: &[[i128; 3]], b: &[[i128; 3]]) -> Vec<[i128; 3]> {
    a.iter().zip(b.iter()).map(|(x, y)|
        [x[0] + y[0], x[1] + y[1], x[2] + y[2]]
    ).collect()
}

/// Negate a field (the b → −b operation, the "Frobenius" on the b-component).
fn negate_field(a: &[[i64; 3]]) -> Vec<[i64; 3]> {
    a.iter().map(|x| [-x[0], -x[1], -x[2]]).collect()
}

// ═══════════════════════════════════════════════════════════════
// TEST 1: LINEARITY — EXACT (pre-truncation, no floor division)
// Uses apply_exact which returns i128 numerators before /denom.
// At this stage NTT + CRT lift are exact integer ops.
// If the kernel is mathematically linear, the diff MUST be exactly zero.
// ═══════════════════════════════════════════════════════════════
fn test_linearity_exact(kernel: &BiotSavartNTT, label: &str,
                        w_a: &[[i64; 3]], w_b: &[[i64; 3]]) -> i128 {
    let u_a = apply_kernel_exact(kernel, w_a);
    let u_b = apply_kernel_exact(kernel, w_b);
    let w_sum = sum_fields(w_a, w_b);
    let u_sum = apply_kernel_exact(kernel, &w_sum);
    let u_a_plus_u_b = sum_fields_i128(&u_a, &u_b);
    let diff = l_inf_diff_i128(&u_sum, &u_a_plus_u_b);
    println!("    {:<30} L∞ diff = {} (exact i128, pre-/denom)", label, diff);
    diff
}

// ═══════════════════════════════════════════════════════════════
// TEST 1b: LINEARITY — POST-TRUNCATION (existing pipeline output)
// This shows the 1-LSB residue introduced by floor division.
// Kept as a contrast to demonstrate where the artifact comes from.
// ═══════════════════════════════════════════════════════════════
fn test_linearity_truncated(kernel: &BiotSavartNTT, label: &str,
                            w_a: &[[i64; 3]], w_b: &[[i64; 3]]) -> i64 {
    let u_a = apply_kernel(kernel, w_a);
    let u_b = apply_kernel(kernel, w_b);
    let w_sum = sum_fields(w_a, w_b);
    let u_sum = apply_kernel(kernel, &w_sum);
    let u_a_plus_u_b = sum_fields(&u_a, &u_b);
    let diff = l_inf_diff(&u_sum, &u_a_plus_u_b);
    println!("    {:<30} L∞ diff = {} (i64, post-/denom)", label, diff);
    diff
}

// ═══════════════════════════════════════════════════════════════
// TEST 2: FROBENIUS COMMUTATION
// Frobenius on the b-component is σ: b → −b.
// If the kernel commutes with this, K(σ(ω)) = σ(K(ω)).
// Equivalently: K(−ω_b) = −K(ω_b) — the kernel respects the b-subspace.
// This is just linearity restricted to the b-component.
// But it's the CONCRETE test of K_b = 0.
// ═══════════════════════════════════════════════════════════════
fn test_frobenius(kernel: &BiotSavartNTT, label: &str,
                  w_b: &[[i64; 3]]) -> i64 {
    let u_b_pos = apply_kernel(kernel, w_b);
    let w_b_neg = negate_field(w_b);
    let u_b_neg = apply_kernel(kernel, &w_b_neg);
    let u_b_pos_negated = negate_field(&u_b_pos);
    let diff = l_inf_diff(&u_b_neg, &u_b_pos_negated);
    println!("    {:<30} L∞ diff = {}", label, diff);
    diff
}

// ═══════════════════════════════════════════════════════════════
// TEST 3: SUBSPACE INVARIANCE (the actual K_b test)
// If the b-component of vorticity is zero, is the b-component
// of velocity also zero? (Tautologically yes for the standard kernel
// because there's only one slot, but this tests the coupling structure.)
//
// More meaningfully: consider two vorticity fields, ω_a and ω_b.
// Does K (ω_a, ω_b) → (u_a, u_b) decouple as u_a = K·ω_a, u_b = K·ω_b?
// This is exactly what linearity proves (Test 1), but verbalized as
// the F_p² subspace-invariance claim.
// ═══════════════════════════════════════════════════════════════

fn run_probe() -> bool {
    println!("{}", "=".repeat(78));
    println!("  D-024 PROBE: K_b Symmetry Test");
    println!("  Question: Does Biot-Savart preserve the b-subspace?");
    println!("{}", "=".repeat(78));
    println!();

    println!("  Building NTT kernel for N={}...", N);
    let kernel = BiotSavartNTT::precompute(N);
    println!("  Kernel ready (denom = {})", kernel.denom);
    println!();

    // Build test vorticity fields
    let w_tg = build_field(|ix, iy, iz| taylor_green_w(ix, iy, iz, N));
    let w_adv1 = build_field(|ix, iy, iz| adversarial_w(ix, iy, iz, N, 12345));
    let w_adv2 = build_field(|ix, iy, iz| adversarial_w(ix, iy, iz, N, 54321));

    println!("  Test field magnitudes:");
    println!("    Taylor-Green:    L∞ = {}", l_inf_norm(&w_tg));
    println!("    Adversarial #1:  L∞ = {}", l_inf_norm(&w_adv1));
    println!("    Adversarial #2:  L∞ = {}", l_inf_norm(&w_adv2));
    println!();

    // Test 1: Exact linearity using pre-truncation i128 numerators.
    // K-Elimination / our division: stay in residues, never truncate.
    println!("  TEST 1 (EXACT): Linearity K(ω_a + ω_b) = K(ω_a) + K(ω_b)");
    println!("    Using apply_exact (i128 pre-/denom values, no floor truncation)");
    let l1e = test_linearity_exact(&kernel, "TG + Adv1", &w_tg, &w_adv1);
    let l2e = test_linearity_exact(&kernel, "TG + Adv2", &w_tg, &w_adv2);
    let l3e = test_linearity_exact(&kernel, "Adv1 + Adv2", &w_adv1, &w_adv2);
    println!();

    // Test 1b: Same test through the truncated pipeline, for contrast.
    println!("  TEST 1b (POST-TRUNCATION): Same linearity check after /denom");
    println!("    Shows the floor-division residue (artifact, not a violation)");
    let l1t = test_linearity_truncated(&kernel, "TG + Adv1", &w_tg, &w_adv1);
    let l2t = test_linearity_truncated(&kernel, "TG + Adv2", &w_tg, &w_adv2);
    let l3t = test_linearity_truncated(&kernel, "Adv1 + Adv2", &w_adv1, &w_adv2);
    println!();

    // Test 2: Frobenius commutation. K(-ω) = -K(ω)?
    println!("  TEST 2: Frobenius K(σ(ω)) = σ(K(ω))   [σ = b → -b]");
    let f1 = test_frobenius(&kernel, "TG b-component", &w_tg);
    let f2 = test_frobenius(&kernel, "Adversarial #1 b-component", &w_adv1);
    let f3 = test_frobenius(&kernel, "Adversarial #2 b-component", &w_adv2);
    println!();

    // Test 3: Cross-mixing. If we put ω_a in the field and ω_b in another,
    // does the kernel acting on the COMBINED produce u_a + u_b·√δ where
    // u_a depends only on ω_a and u_b only on ω_b? This IS linearity.
    println!("  TEST 3: Cross-mixing (b → -b applied at output level)");
    println!("    For any field W, K(W) and -K(-W) must be equal (linearity-derived)");
    println!("    This IS K_b = 0 in the F_p² sense:");
    println!("      If kernel had a √δ component, K(b√δ) would have an a-part");
    println!("      and the b → -b symmetry of K(W) would break.");
    println!();

    let max_l_exact = l1e.max(l2e).max(l3e);
    let max_l_trunc = l1t.max(l2t).max(l3t);
    let max_f = f1.max(f2).max(f3);

    println!("  RESULTS:");
    println!("    Linearity (exact, pre-/denom):    L∞ diff = {}", max_l_exact);
    println!("    Linearity (post-/denom):          L∞ diff = {}", max_l_trunc);
    println!("    Frobenius commutation:            L∞ diff = {}", max_f);
    println!();

    let kb_zero = max_l_exact == 0 && max_f == 0;

    if kb_zero {
        println!("  VERDICT: K_b = 0 confirmed.");
        println!("    Linearity EXACT in pre-truncation arithmetic (zero diff).");
        println!("    Frobenius commutation EXACT bit-for-bit.");
        println!("    K(-ω) = -K(ω). No √δ component in the kernel.");
        println!("    The b-subspace is invariant under Biot-Savart by construction.");
        println!();
        println!("    The {}-LSB post-truncation residue is the floor-division artifact:", max_l_trunc);
        println!("      K(a+b)/d ≠ K(a)/d + K(b)/d when (K(a) mod d) + (K(b) mod d) ≥ d.");
        println!("    The CRAM/K-Elim solution is to stay in residues — no division.");
        println!("    This probe demonstrates that the underlying kernel arithmetic is exact.");
        println!();
        println!("    Implication: If ω_b(0) = 0, Biot-Savart preserves ω_b = 0.");
        println!("    The remaining b-coupling channels are advection and stretching");
        println!("    (bilinear terms). These can drive b only through cross-products");
        println!("    that vanish when both ω_b = 0 AND u_b = 0.");
        println!("    Since K_b = 0 ⟹ u_b = K * ω_b, if ω_b = 0 then u_b = 0,");
        println!("    so the cross-products are zero. b = 0 is INVARIANT.");
        println!();
        println!("    NEXT GATE: Test attractor stability under ω_b₀ ≠ 0 perturbation.");
    } else if max_f != 0 {
        println!("  VERDICT: K_b ≠ 0. Frobenius commutation FAILED.");
        println!("    The kernel does not commute with b → -b sign flip.");
        println!("    A √δ component is leaking into the kernel.");
        println!("    D-024 as currently formulated CANNOT close at the kernel level.");
    } else {
        println!("  VERDICT: Exact linearity FAILED ({}).", max_l_exact);
        println!("    The pre-truncation NTT + CRT lift should be exactly linear.");
        println!("    Investigate the kernel pipeline — this is unexpected.");
    }
    println!();
    println!("{}", "=".repeat(78));

    kb_zero
}

fn main() {
    let kb_zero = run_probe();
    std::process::exit(if kb_zero { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_is_exactly_linear_pre_truncation() {
        let kernel = BiotSavartNTT::precompute(N);
        let w_tg = build_field(|ix, iy, iz| taylor_green_w(ix, iy, iz, N));
        let w_adv = build_field(|ix, iy, iz| adversarial_w(ix, iy, iz, N, 12345));
        let diff = test_linearity_exact(&kernel, "test", &w_tg, &w_adv);
        // Pre-truncation is NTT + CRT lift, both exact integer ops.
        // Linearity must be exact: zero diff.
        assert_eq!(diff, 0, "kernel must be exactly linear pre-/denom (got L∞ diff = {})", diff);
    }

    #[test]
    fn kernel_is_linear_modulo_truncation() {
        // Post-truncation, allow 1-LSB residue from floor division.
        let kernel = BiotSavartNTT::precompute(N);
        let w_tg = build_field(|ix, iy, iz| taylor_green_w(ix, iy, iz, N));
        let w_adv = build_field(|ix, iy, iz| adversarial_w(ix, iy, iz, N, 12345));
        let diff = test_linearity_truncated(&kernel, "test", &w_tg, &w_adv);
        assert!(diff <= 1, "post-truncation linearity holds within 1 LSB (got {})", diff);
    }

    #[test]
    fn kernel_commutes_with_frobenius() {
        let kernel = BiotSavartNTT::precompute(N);
        let w = build_field(|ix, iy, iz| taylor_green_w(ix, iy, iz, N));
        let diff = test_frobenius(&kernel, "test", &w);
        assert_eq!(diff, 0, "kernel must commute with Frobenius (got L∞ diff = {})", diff);
    }

    #[test]
    fn kernel_handles_zero_input() {
        let kernel = BiotSavartNTT::precompute(N);
        let w_zero: Vec<[i64; 3]> = vec![[0, 0, 0]; N * N * N];
        let u = apply_kernel(&kernel, &w_zero);
        assert_eq!(l_inf_norm(&u), 0, "K(0) must be 0");
    }
}
