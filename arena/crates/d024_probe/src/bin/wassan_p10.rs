//! # Push 10: WassanVortField on Push 9's saturating ICs (safe-snapshot)
//!
//! Push 9 ran the canonical N=8 NS solver on TG/ABC/Random ICs at CFL_max.
//! TG contracted cleanly; ABC and Random saturated to i64::MAX within ~10
//! steps and were silently absorbed by the clamp at `ns_v5/src/ns3d.rs:730`.
//!
//! Push 10 uses the right substrate: `WassanVortField` with Recumbent
//! cells (CRTBigInt-backed, ~2^504 capacity per stack). Each cell can
//! exit the primary corridor without saturating — it goes to overflow,
//! then to hot, all tracked exactly. `overflow_steps` accumulates as the
//! longitudinal excursion record.
//!
//! The previous Push 10 attempt panicked on `WassanVortField::snapshot()`
//! for ABC because that method calls `reconstruct_exact()` → `to_i64_exact()`
//! and panics when hot-lane values exceed i64. The DATA was preserved
//! exactly in CRTBigInt lanes; only the diagnostic readout failed.
//!
//! Fix: `safe_snapshot()` (added to ns3d_wassan.rs). Returns `SafeSnapshot`
//! with tier counts (already in primitive types), `overflow_steps` (u64
//! direct), and H² norm in CRTBigInt lane residues (no reconstruction).
//! Never panics.
//!
//! What this push answers:
//!   - For ABC/Random at amp=10000 where Push 9 saturated, what does
//!     the Recumbent substrate actually carry?
//!   - Does `overflow_steps` stay bounded over a long horizon (corridor
//!     control verified) or grow unboundedly (blowup signalled exactly)?

use ns_regularity::ns3d::{N, NsParams, Vort3D};
use ns_regularity::ns3d_wassan::WassanVortField;

const N_STEPS: usize = 100;

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

fn run_traj(label: &str, w0: Vort3D, params: &NsParams, n_steps: usize) {
    println!("── {label} ──");
    let mut wf = WassanVortField::from_vort3d(&w0);

    let snap0 = wf.safe_snapshot();
    println!("  Initial:  P:{:>4} O:{:>3} H:{:>4} | max_ovstep={} | linf={} | h2_lane0={}",
             snap0.primary_cells, snap0.overflow_cells, snap0.hot_cells,
             snap0.max_overflow_steps, snap0.max_linf_i128, snap0.h2_norm_lane0);

    let mut max_ov_seen = snap0.max_overflow_steps;
    let mut snap_history: Vec<(usize, ns_regularity::ns3d_wassan::SafeSnapshot)> = Vec::new();
    snap_history.push((0, snap0));

    for step in 0..n_steps {
        wf.ns_step_wassan(params);
        let snap = wf.safe_snapshot();
        max_ov_seen = max_ov_seen.max(snap.max_overflow_steps);

        let report_at = step == 0 || step == 4 || step == 9 || step == 24
                     || step == 49 || step == 99 || step == n_steps - 1;
        if report_at {
            println!("  step {:>3}: P:{:>4} O:{:>3} H:{:>4} | max_ovstep={:>4} | linf={} | h2_lane0={}",
                     step + 1, snap.primary_cells, snap.overflow_cells,
                     snap.hot_cells, snap.max_overflow_steps, snap.max_linf_i128, snap.h2_norm_lane0);
            snap_history.push((step + 1, snap));
        }
    }

    let final_snap = &snap_history.last().unwrap().1;
    let in_corridor_pct = (final_snap.primary_cells * 100) / 1536;
    println!("  Summary: max_overflow_steps_seen={}  primary_at_end={}/1536 ({}%)  fully_in_corridor={}",
             max_ov_seen, final_snap.primary_cells, in_corridor_pct, final_snap.fully_in_corridor());

    // overflow_steps trajectory — the longitudinal observable
    let ov_curve: Vec<u64> = snap_history.iter().map(|(_, s)| s.max_overflow_steps).collect();
    println!("  max_overflow_steps trajectory: {:?}", ov_curve);
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  PUSH 10: WassanVortField on Push 9 saturating ICs (safe-snapshot)");
    println!("  Substrate: WassanRecord(Recumbent(CRTBigInt × 8 lanes))");
    println!("  Each cell: ~2^504 capacity per stack — no clamping, no saturation");
    println!("  Diagnostic: safe_snapshot() — never reconstructs to i64, never panics");
    println!("  N={}, N_STEPS={}", N, N_STEPS);
    println!("{}", "=".repeat(78));
    println!();

    let nu_scaled = 1_000_000i64;
    let params = NsParams::cfl_max(nu_scaled);
    println!("  NsParams::cfl_max({}): nu_scaled={}, dt_latt={}, CFL ok: {}",
             nu_scaled, params.nu_scaled, params.dt_latt, params.cfl_ok());
    println!();
    println!("  Cell tier discipline:");
    println!("    primary lane (CRTBigInt, |v| < primary_radius)");
    println!("    overflow lane (CRTBigInt, demoted from primary on excursion)");
    println!("    hot lane (CRTBigInt, K-Elim phase-locked, very large excursions)");
    println!("    overflow_steps: u64 — longitudinal excursion counter (monotone +)");
    println!();
    println!("{}", "-".repeat(78));
    println!();

    run_traj("TG amp=10000",     Vort3D::taylor_green(10000), &params, N_STEPS);
    run_traj("TG amp=1000",      Vort3D::taylor_green(1000),  &params, N_STEPS);
    run_traj("ABC amp=10000",    Vort3D::abc_flow(10000),     &params, N_STEPS);
    run_traj("ABC amp=1000",     Vort3D::abc_flow(1000),      &params, N_STEPS);
    run_traj("Random amp=10000", random_field(99, 10000),     &params, N_STEPS);
    run_traj("Random amp=1000",  random_field(99, 1000),      &params, N_STEPS);

    println!("{}", "=".repeat(78));
    println!("  READING");
    println!("{}", "=".repeat(78));
    println!();
    println!("  Push 9 (raw Vort3D + ns_step):");
    println!("    TG: contracted cleanly (factor 0.913/step)");
    println!("    ABC, Random amp=10000: saturated to i64::MAX in ~10 steps");
    println!("                           (clamp at ns3d.rs:730 absorbed silently)");
    println!();
    println!("  Push 10 (WassanVortField + ns_step_wassan + safe_snapshot):");
    println!("    Each cell exits primary into overflow then hot — no clamping.");
    println!("    The Recumbent substrate carries the trajectory exactly.");
    println!();
    println!("    overflow_steps trajectory tells corridor-control story:");
    println!("      bounded → field stays in corridor (T-NDO satisfied)");
    println!("      grows linearly → cells stay in overflow but don't escape");
    println!("      grows unboundedly → blowup signalled exactly");
    println!();
    println!("    H² norm tracked as CRTBigInt lane residues — capacity 2^504 per");
    println!("    stack, never saturates. Lane 0 residue gives a fingerprint of");
    println!("    the H² magnitude evolution without reconstructing to i64.");
    println!();
    println!("{}", "=".repeat(78));
}
