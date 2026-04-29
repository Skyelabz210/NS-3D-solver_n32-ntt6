//! # Push 9: Canonical Workspace Solver Comparison
//!
//! Replaces Push 7/8 broken CRTBigInt experiments with the workspace's
//! canonical NS solver. The canonical solver uses:
//!
//! - 4 NTT primes {73, 89, 97, 113}, NTT_MODULUS ≈ 7×10⁷
//! - Iterated K-Elimination (`garner4`) for Biot-Savart reconstruction
//!   (signed centered i64 by construction, fits in i64 because
//!   centered-range M/2 ≈ 3.6×10⁷ > N³ × NS_SCALE = 5.1×10⁶)
//! - i128 intermediates, ordinary truncating integer division `/` for
//!   `nu·lap/SCALE²` and `dt·rhs/N²` — this is K-Elim's piggy-back
//!   division: once K-Elim returns a centered i64, the reduction
//!   to physical units is just integer truncation
//! - CFL-bounded params: `NsParams::cfl_max(nu_scaled)` chooses the
//!   maximum dt that satisfies `6·nu·dt ≤ SCALE²·N²`
//!
//! No modular-inverse division, no separate sign field, no
//! reconstruction-with-wrapping. The K-Elim/Garner4 substrate IS the
//! signed integer reconstruction Push 7/8 was missing.
//!
//! Push 7/8 retraction:
//! - Push 7's "T4=T1-T3 falsifies extension instability": invalid (broken arithmetic)
//! - Push 8's "canonical = b-step growth": invalid (same broken arithmetic both sides)
//!
//! This push: run the canonical solver on the same TG/ABC/Random ICs.
//! Report what actually happens to L² and L∞ over 100 steps at canonical
//! CFL params. No claims about D-024 until after this.

use ns_regularity::ns3d::{
    N, NS_SCALE, Vort3D, NsParams, ns_step, l_inf, l2_sq,
};

const N_STEPS: u64 = 100;

fn taylor_green(amp: i64) -> Vort3D {
    Vort3D::taylor_green(amp)
}

fn abc_field(amp: i64) -> Vort3D {
    Vort3D::abc_flow(amp)
}

fn random_field(seed: u64, amp: i64) -> Vort3D {
    let mut wx = [[[0i64; N]; N]; N];
    let mut wy = [[[0i64; N]; N]; N];
    let mut wz = [[[0i64; N]; N]; N];
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bg { TG, ABC, Random }

struct TrajResult {
    label: String,
    initial_l2: i128,
    initial_linf: i64,
    l2_traj: Vec<i128>,
    linf_traj: Vec<i64>,
    final_l2: i128,
    final_linf: i64,
    saturated: bool,  // true if any cell hit i64::MAX/MIN clamp
}

fn run_canonical(label: String, w0: Vort3D, params: &NsParams, n_steps: u64) -> TrajResult {
    let mut w = w0.clone();
    let initial_l2 = l2_sq(&w);
    let initial_linf = l_inf(&w);
    let mut l2_traj = Vec::with_capacity(n_steps as usize + 1);
    let mut linf_traj = Vec::with_capacity(n_steps as usize + 1);
    l2_traj.push(initial_l2);
    linf_traj.push(initial_linf);

    let mut saturated = false;

    for _ in 0..n_steps {
        w = ns_step(&w, params);
        let cur_linf = l_inf(&w);
        let cur_l2 = l2_sq(&w);
        l2_traj.push(cur_l2);
        linf_traj.push(cur_linf);
        // Saturating clamp at i64::MAX is a sentinel — if any cell touches
        // i64::MAX/2 we know the integration is approaching the corridor edge
        if cur_linf > i64::MAX / 4 {
            saturated = true;
        }
    }

    let final_l2 = l2_traj[l2_traj.len() - 1];
    let final_linf = linf_traj[linf_traj.len() - 1];

    TrajResult {
        label, initial_l2, initial_linf,
        l2_traj, linf_traj,
        final_l2, final_linf, saturated,
    }
}

fn print_result(r: &TrajResult) {
    println!("  {}", r.label);
    println!("    Initial L²:   {}", r.initial_l2);
    println!("    Final   L²:   {}", r.final_l2);
    println!("    Initial L∞:   {}", r.initial_linf);
    println!("    Final   L∞:   {}", r.final_linf);
    println!("    Saturated (any cell > i64::MAX/4): {}", r.saturated);

    // Per-step L² ratio (multiplicative growth/decay rate)
    if r.initial_l2 > 0 && r.final_l2 > 0 {
        let log10_ratio = (r.final_l2 as f64 / r.initial_l2 as f64).log10();
        let per_step = log10_ratio / r.l2_traj.len().saturating_sub(1).max(1) as f64;
        let factor = 10f64.powf(per_step);
        println!("    L² total log10 ratio: {:.4}", log10_ratio);
        println!("    L² per-step factor:   {:.6}  (per step)", factor);
    }

    // Print L² trajectory at sparse points
    let n = r.l2_traj.len();
    let pts: Vec<(usize, i128)> = (0..n).step_by(10.max(n / 11))
        .chain(std::iter::once(n - 1))
        .map(|i| (i, r.l2_traj[i]))
        .collect();
    print!("    L² snapshots: ");
    for (i, v) in pts {
        print!("[{}: {:.2e}] ", i, v as f64);
    }
    println!();

    // L∞ snapshots
    let pts: Vec<(usize, i64)> = (0..n).step_by(10.max(n / 11))
        .chain(std::iter::once(n - 1))
        .map(|i| (i, r.linf_traj[i]))
        .collect();
    print!("    L∞ snapshots: ");
    for (i, v) in pts {
        print!("[{}: {:.2e}] ", i, v as f64);
    }
    println!();
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  PUSH 9: CANONICAL N=8 SOLVER on Push 7/8 ICs");
    println!("  Solver: ns_v5::ns3d::ns_step  (K-Elim/Garner4 + truncating division)");
    println!("  Substrate: 4 NTT primes {{73,89,97,113}}, M = 71,213,617");
    println!("  No CRTBigInt-with-sign-field. No modular-inverse pseudo-division.");
    println!("  N = {}, NS_SCALE = {}, N_STEPS = {}", N, NS_SCALE, N_STEPS);
    println!("{}", "=".repeat(78));
    println!();

    let nu_scaled = 1_000_000i64;
    let params = NsParams::cfl_max(nu_scaled);
    println!("  NsParams::cfl_max({}): nu_scaled={}, dt_latt={}",
             nu_scaled, params.nu_scaled, params.dt_latt);
    println!("  CFL ok: {}", params.cfl_ok());
    println!();
    println!("  Note: dt_latt = (SCALE²·N²)/(6·nu) = (10⁸·64)/(6·10⁶) = {}",
             params.dt_latt);
    println!("  Push 7/8 used dt_latt=100 — that was 10× SMALLER than CFL bound,");
    println!("  not 10× larger. Push 8's 'CFL violation' framing was inverted.");
    println!();

    println!("{}", "-".repeat(78));
    println!("  Trajectories at CFL_max parameters");
    println!("{}", "-".repeat(78));
    println!();

    let cases = vec![
        ("TG amp=10000",        taylor_green(10000)),
        ("TG amp=1000",         taylor_green(1000)),
        ("TG amp=100",          taylor_green(100)),
        ("ABC amp=10000",       abc_field(10000)),
        ("ABC amp=100",         abc_field(100)),
        ("Random amp=10000",    random_field(99, 10000)),
        ("Random amp=100",      random_field(99, 100)),
    ];

    let mut results = Vec::new();
    for (label, w0) in cases {
        let r = run_canonical(label.to_string(), w0, &params, N_STEPS);
        print_result(&r);
        results.push(r);
    }

    println!("{}", "-".repeat(78));
    println!("  Comparison vs Push 7/8 b-step claims");
    println!("{}", "-".repeat(78));
    println!();
    println!("  Push 7/8 reported: log10/step = 0.31 across all backgrounds, b-step.");
    println!("  Canonical solver   per-step factors above (L², 100 steps).");
    println!();
    println!("  Reading:");
    println!("  - factor < 1.0:  L² decreases — bounded/contracting");
    println!("  - factor ≈ 1.0:  L² roughly conserved");
    println!("  - factor > 1.0:  L² grows  (with no saturation: physical;");
    println!("                              with saturation: integrator left corridor)");
    println!();
    println!("  If canonical solver shows bounded L² where Push 7/8 showed 0.31/step,");
    println!("  that confirms Push 7/8 growth was an arithmetic artifact of the");
    println!("  CRTBigInt sign+modular-inverse implementation, not the integrator");
    println!("  or the extension.");
}
