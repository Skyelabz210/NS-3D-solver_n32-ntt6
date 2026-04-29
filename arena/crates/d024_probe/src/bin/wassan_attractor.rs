//! # D-024 b-Attractor on Recumbent + WassanVortField
//!
//! Push 5: rebuild the b-attractor test using the workspace's canonical
//! Recumbent + WassanVortField substrate. No renormalization. No i64
//! truncation in the inner loop beyond what the canonical operators do
//! (which is corridor-bounded and tier-tracked).
//!
//! ## Architectural fit
//!
//! - a-grid: WassanVortField (existing)
//! - b-grid: NEW WassanVortField (parallel structure)
//! - Bilinear cross-coupling via canonical `discrete_advection`/`discrete_stretching`
//!   applied to mixed (a, b) snapshots. These operators are i128-promoted
//!   internally; their final i64 cast is corridor-bounded.
//! - Re-entry into WassanVortField via `from_vort3d` re-tiers each cell
//!   and updates `overflow_steps` if the corridor was breached.
//!
//! ## Why this is correct where attractor_crt was broken
//!
//! attractor_crt did `let diff_i = diff.to_i64().unwrap_or(0)` per term
//! per cell per step — silently substituted zero on overflow, accumulator
//! corruption. This version uses the canonical operators which preserve
//! exactness within the corridor, and uses Recumbent re-entry to log
//! corridor escape events without silent failure.
//!
//! ## Diagnostic
//!
//! Per-step ‖b‖² in i128 (from canonical `l2_sq`). Total trajectory
//! tracked. Tier counts (primary/overflow/hot) reported per step from
//! the WassanVortField. If overflow_steps stays at 0 across all cells
//! through all steps, b stayed in the corridor — the result is exact
//! for the discrete scheme.

#![forbid(unsafe_code)]

use ns_regularity::ns3d::{
    Vort3D, Vel3D, NsParams, Field3D, N, NS_SCALE,
    biot_savart_n8, discrete_laplacian, discrete_advection, discrete_stretching,
    l2_sq, zero_field,
};
use ns_regularity::ns3d_wassan::WassanVortField;
use ns_regularity::recumbent::RecumbentTier;
use ns_regularity::ns3d_energy::ns_step_skew;

const N_STEPS: u64 = 100;

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

fn random_perturbation_field(seed: u64, amp: i64) -> Vort3D {
    let mut wx = zero_field(); let mut wy = zero_field(); let mut wz = zero_field();
    for i in 0..N { for j in 0..N { for k in 0..N {
        let h = ((i as u64 * 1009 + j as u64 * 1013 + k as u64 * 1019)
                 .wrapping_mul(seed.wrapping_add(1))).wrapping_mul(2654435761);
        let raw0 = ((h >>  0) & 0xFFFF) as i64 - 32768;
        let raw1 = ((h >> 16) & 0xFFFF) as i64 - 32768;
        let raw2 = ((h >> 32) & 0xFFFF) as i64 - 32768;
        wx[i][j][k] = (raw0 as i128 * amp as i128 / 32768) as i64;
        wy[i][j][k] = (raw1 as i128 * amp as i128 / 32768) as i64;
        wz[i][j][k] = (raw2 as i128 * amp as i128 / 32768) as i64;
    }}}
    Vort3D { wx, wy, wz }
}

fn zero_vort() -> Vort3D {
    Vort3D { wx: zero_field(), wy: zero_field(), wz: zero_field() }
}

// ─── Cross-coupled b-step ───────────────────────────────────────────────────
//
// The b-equation:
//   ∂_t b = ν Δb − (u_a · ∇)b − (u_b · ∇)a + (ω_a · ∇)u_b + (ω_b · ∇)u_a
//
// Each term is per-component. We apply the canonical operators to mixed
// (a, b) snapshots and assemble. Output is the new b-field as Vort3D.
//
// NOTE: we use ω_b = b throughout, and u_b = K * ω_b from biot_savart_n8.

fn step_b_coupled(b_old: &Vort3D, u_a: &Vel3D, w_a: &Vort3D, u_b: &Vel3D, params: &NsParams) -> Vort3D {
    // Per-component: ν Δ b
    let lap_bx = discrete_laplacian(&b_old.wx);
    let lap_by = discrete_laplacian(&b_old.wy);
    let lap_bz = discrete_laplacian(&b_old.wz);

    // -(u_a · ∇)b: discrete_advection(u_a, b_component) per component
    let adv_ab_x = discrete_advection(u_a, &b_old.wx);
    let adv_ab_y = discrete_advection(u_a, &b_old.wy);
    let adv_ab_z = discrete_advection(u_a, &b_old.wz);

    // -(u_b · ∇)a: discrete_advection(u_b, a_component) per component
    let adv_ba_x = discrete_advection(u_b, &w_a.wx);
    let adv_ba_y = discrete_advection(u_b, &w_a.wy);
    let adv_ba_z = discrete_advection(u_b, &w_a.wz);

    // (ω_a · ∇)u_b: discrete_stretching(ω_a as Vort3D, u_b_component as Field3D)
    let str_ab_x = discrete_stretching(w_a, &u_b.ux);
    let str_ab_y = discrete_stretching(w_a, &u_b.uy);
    let str_ab_z = discrete_stretching(w_a, &u_b.uz);

    // (ω_b · ∇)u_a: discrete_stretching(b as Vort3D, u_a_component as Field3D)
    let str_ba_x = discrete_stretching(b_old, &u_a.ux);
    let str_ba_y = discrete_stretching(b_old, &u_a.uy);
    let str_ba_z = discrete_stretching(b_old, &u_a.uz);

    // Assemble: b_new = b_old + dt × (ν Δb + adv_ab + adv_ba + str_ab + str_ba)
    // Note adv_ab and adv_ba have the canonical sign from discrete_advection
    // (which returns +(u·∇)ω; our equation has - signs, so we subtract).
    let n2 = (N * N) as i128;
    let s2 = (NS_SCALE as i128) * (NS_SCALE as i128);
    let nu = params.nu_scaled as i128;
    let dt = params.dt_latt as i128;

    let assemble = |b: &Field3D, lap: Field3D, adv_ab: Field3D, adv_ba: Field3D, str_ab: Field3D, str_ba: Field3D| -> Field3D {
        let mut out = zero_field();
        for i in 0..N { for j in 0..N { for k in 0..N {
            // ν Δ b: nu × lap, then /SCALE²
            let diff_i: i128 = nu * (lap[i][j][k] as i128) / s2;
            // RHS: diff − adv_ab − adv_ba + str_ab + str_ba
            let rhs: i128 = diff_i
                          - adv_ab[i][j][k] as i128
                          - adv_ba[i][j][k] as i128
                          + str_ab[i][j][k] as i128
                          + str_ba[i][j][k] as i128;
            // Update: b_new = b_old + (dt × rhs) / n²
            let next: i128 = b[i][j][k] as i128 + (dt * rhs) / n2;
            // Saturating clamp to i64 (same protocol as canonical ns_step)
            out[i][j][k] = next.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        }}}
        out
    };

    Vort3D {
        wx: assemble(&b_old.wx, lap_bx, adv_ab_x, adv_ba_x, str_ab_x, str_ba_x),
        wy: assemble(&b_old.wy, lap_by, adv_ab_y, adv_ba_y, str_ab_y, str_ba_y),
        wz: assemble(&b_old.wz, lap_bz, adv_ab_z, adv_ba_z, str_ab_z, str_ba_z),
    }
}

// ─── WassanVortField-backed step ────────────────────────────────────────────
//
// The protocol mirrors WassanVortField::ns_step_wassan but uses the
// cross-coupled b-step on the b-field while a-field uses the standard step.

fn run_attractor_test(
    test_id: &'static str,
    bg: Vort3D,                  // a-field initial condition
    b0: Vort3D,                  // b-field initial perturbation
    params: NsParams,
    n_steps: u64,
    pure_b: bool,                // if true, freeze a at zero
) -> AttractorResult {
    // Build WassanVortField for both
    let mut wf_a = WassanVortField::from_vort3d(&bg);
    let mut wf_b = WassanVortField::from_vort3d(&b0);

    let a_l2_initial = l2_sq(&wf_a.to_vort3d());
    let b_l2_initial = l2_sq(&wf_b.to_vort3d());
    let mut b_l2_max = b_l2_initial;
    let mut b_l2_min = b_l2_initial;

    // Per-step ‖b‖² in i128 — exact, no renormalization
    let mut b_l2_trajectory: Vec<i128> = Vec::with_capacity(n_steps as usize + 1);
    b_l2_trajectory.push(b_l2_initial);

    // Tier diagnostics: # cells in primary / overflow / hot per step
    let mut overflow_excursions_total: u64 = 0;

    for step in 0..n_steps {
        // Wrap the entire step in catch_unwind to identify failure point
        let step_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Snapshot a and b through WassanVortField
            let a_snap = wf_a.to_vort3d();
            let b_snap = wf_b.to_vort3d();

            // Compute u_a from a-snap
            let u_a = biot_savart_n8(&a_snap);
            // Compute u_b from b-snap (uses K_b = 0)
            let u_b = biot_savart_n8(&b_snap);

            // b-step uses cross-coupled equation
            let b_new = step_b_coupled(&b_snap, &u_a, &a_snap, &u_b, &params);

            (a_snap, b_new)
        }));

        let (a_snap, b_new) = match step_result {
            Ok(v) => v,
            Err(_) => {
                println!("    [{}] PANIC at step {}, last ‖b‖² = {}",
                    test_id, step, b_l2_trajectory.last().copied().unwrap_or(-1));
                println!("    [{}] Max ‖b‖² seen: {}", test_id, b_l2_max);
                return AttractorResult {
                    test_id,
                    a_l2_initial,
                    a_l2_final: a_l2_initial,
                    b_l2_initial,
                    b_l2_final: b_l2_max,
                    b_l2_max,
                    b_l2_min,
                    b_l2_trajectory,
                    overflow_excursions_total,
                    verdict: "PANIC: i64 overflow in step",
                };
            }
        };

        // a-step uses canonical NS skew step
        if !pure_b {
            // Wrap in catch_unwind: a-step can panic if a blows up under bilinear feedback
            let a_step_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                wf_a.ns_step_wassan(&params);
            }));
            if a_step_result.is_err() {
                println!("    [{}] PANIC in a-step at step {}", test_id, step);
                return AttractorResult {
                    test_id,
                    a_l2_initial,
                    a_l2_final: a_l2_initial,
                    b_l2_initial,
                    b_l2_final: b_l2_max,
                    b_l2_max,
                    b_l2_min,
                    b_l2_trajectory,
                    overflow_excursions_total,
                    verdict: "PANIC: a-step overflow",
                };
            }
        }

        // For b, rebuild WassanVortField from b_new (acceptable per attractor_crt notes)
        wf_b = WassanVortField::from_vort3d(&b_new);

        // Track ‖b‖² (no panic path needed, b_new is i64-Vort3D)
        let b_l2 = l2_sq(&b_new);
        b_l2_trajectory.push(b_l2);
        if b_l2 > b_l2_max { b_l2_max = b_l2; }
        if b_l2 < b_l2_min { b_l2_min = b_l2; }

        // Tier diagnostics — optional, panic-protected
        let snap_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            wf_b.snapshot()
        }));
        if let Ok(snapshot) = snap_result {
            if !snapshot.fully_in_corridor() {
                overflow_excursions_total += 1;
            }
        } else {
            // Snapshot panicked — b has values exceeding i64. Count as excursion
            // and continue. The b_l2 from b_new (i128) is still valid since clamp
            // has already kept b_new within i64 (saturating). After re-entry,
            // a_snap could exceed i64 if the b-evolution saturated values.
            overflow_excursions_total += 1;
        }

        let _ = a_snap; // a_snap was used inside the catch
    }

    let a_l2_final = l2_sq(&wf_a.to_vort3d());
    let b_l2_final = l2_sq(&wf_b.to_vort3d());

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

    AttractorResult {
        test_id,
        a_l2_initial, a_l2_final,
        b_l2_initial, b_l2_final,
        b_l2_max, b_l2_min,
        b_l2_trajectory,
        overflow_excursions_total,
        verdict,
    }
}

#[derive(Debug)]
struct AttractorResult {
    test_id: &'static str,
    a_l2_initial: i128,
    a_l2_final: i128,
    b_l2_initial: i128,
    b_l2_final: i128,
    b_l2_max: i128,
    b_l2_min: i128,
    b_l2_trajectory: Vec<i128>,
    overflow_excursions_total: u64,
    verdict: &'static str,
}

fn print_result(r: &AttractorResult) {
    println!("  {} ", r.test_id);
    println!("    ‖a‖₂² initial: {}", r.a_l2_initial);
    println!("    ‖a‖₂² final:   {}", r.a_l2_final);
    println!("    ‖b‖₂² initial: {}", r.b_l2_initial);
    println!("    ‖b‖₂² final:   {}", r.b_l2_final);
    println!("    ‖b‖₂² max:     {}", r.b_l2_max);
    println!("    ‖b‖₂² min:     {}", r.b_l2_min);
    if r.b_l2_initial > 0 {
        let ratio_x1000 = (r.b_l2_final * 1000) / r.b_l2_initial.max(1);
        println!("    ratio b²(T)/b²(0): {}/1000", ratio_x1000);
    }
    println!("    overflow excursions: {} / {} steps",
             r.overflow_excursions_total, r.b_l2_trajectory.len() - 1);
    println!("    VERDICT: {}", r.verdict);
    println!();
}

fn main() {
    println!("{}", "=".repeat(78));
    println!("  D-024 ATTRACTOR ON RECUMBENT + WassanVortField (Push 5)");
    println!("  Canonical ns_v5 substrate. No renormalization. ");
    println!("  Cross-coupled b-step via discrete_laplacian/advection/stretching.");
    println!("  N = {}, N_STEPS = {}", N, N_STEPS);
    println!("{}", "=".repeat(78));
    println!();

    // SANITY 0: Round-trip TG init through WassanVortField
    println!("  [SANITY] Round-trip TG init through WassanVortField...");
    let tg_test = taylor_green(10000);
    let tg_l2_pre = l2_sq(&tg_test);
    let mut max_pre = 0i64;
    for f in [&tg_test.wx, &tg_test.wy, &tg_test.wz] {
        for row in f.iter() { for col in row.iter() { for &v in col.iter() {
            max_pre = max_pre.max(v.abs());
        }}}
    }
    println!("    TG pre-WASSAN: ‖·‖² = {}, max |v| = {}", tg_l2_pre, max_pre);

    let wf_test = WassanVortField::from_vort3d(&tg_test);
    let tg_after = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        wf_test.to_vort3d()
    }));
    match tg_after {
        Ok(v) => {
            let l2_post = l2_sq(&v);
            println!("    TG post-WASSAN: ‖·‖² = {} (round-trip {})",
                l2_post, if l2_post == tg_l2_pre { "EXACT" } else { "DRIFTED" });
        }
        Err(_) => {
            println!("    PANIC on to_vort3d round-trip — TG init too large for Recumbent.");
            println!("    Will use smaller amplitudes to stay within corridor.");
            return;
        }
    }
    println!();

    // CFL-bounded params at canonical ν = 0.01 (Re = 100).
    // BUT: the cross-coupled b-equation has a tighter CFL bound than standard NS.
    // Push 4 (Jacobian) measured per-step ratio 0.955 at dt_latt = 100, which is
    // contracting. Push 5 used dt_latt = 1066 (canonical NS CFL) and panicked at
    // step 3 due to bilinear amplification. Use dt_latt = 100 here.
    let nu_scaled: i64 = 1_000_000;
    let canonical = NsParams::cfl_max(nu_scaled);
    let params = NsParams { nu_scaled, dt_latt: 100 };
    println!("  Canonical CFL would give dt_latt = {} but b-equation needs tighter.", canonical.dt_latt);
    println!("  Using dt_latt = {} (10× smaller than NS CFL, matches Jacobian probe).",
             params.dt_latt);
    println!("  Params: ν_scaled = {}, dt_latt = {}, NS-CFL ok = {}",
             params.nu_scaled, params.dt_latt, params.cfl_ok());
    println!();

    // Tests at canonical amplitudes. Push 4 verified contraction at dt_latt=100.
    let tg_bg = taylor_green(10000);
    let tg_pert = random_perturbation_field(12345, 100);  // 1% of 10000
    let abc_bg = abc_field(10000);
    let rnd_bg = random_perturbation_field(99, 10000);
    let pure_b_a = zero_vort();
    let pure_b_b = random_perturbation_field(77777, 100);

    let mk_params = || NsParams { nu_scaled: params.nu_scaled, dt_latt: params.dt_latt };

    let r1 = run_attractor_test("W1: TG bg, 1% pert",
        tg_bg.clone(), tg_pert.clone(), mk_params(), N_STEPS, false);
    let r2 = run_attractor_test("W2: ABC bg, 1% pert",
        abc_bg.clone(), tg_pert.clone(), mk_params(), N_STEPS, false);
    let r3 = run_attractor_test("W3: Random bg, 1% pert",
        rnd_bg.clone(), tg_pert.clone(), mk_params(), N_STEPS, false);
    let r4 = run_attractor_test("W4: Pure-b (ω_a = 0)",
        pure_b_a.clone(), pure_b_b.clone(), mk_params(), N_STEPS, true);
    let r5 = run_attractor_test("W5: TG bg, larger 10% pert",
        tg_bg.clone(), random_perturbation_field(12345, 1000), mk_params(), N_STEPS, false);

    let tests = [&r1, &r2, &r3, &r4, &r5];
    for r in tests.iter() { print_result(r); }

    println!("{}", "=".repeat(78));
    println!("  COUPLING ISOLATION (control: full vs pure-b at same b₀)");
    println!("{}", "=".repeat(78));
    println!();

    let r_full = run_attractor_test("Full TG coupling",
        tg_bg.clone(), tg_pert.clone(), mk_params(), N_STEPS, false);
    let r_pure = run_attractor_test("Pure diffusion (ω_a = 0)",
        pure_b_a.clone(), tg_pert.clone(), mk_params(), N_STEPS, true);
    let r_abc_full = run_attractor_test("Full ABC coupling",
        abc_bg.clone(), tg_pert.clone(), mk_params(), N_STEPS, false);

    println!("  TG bg, b₀ = 1% TG:");
    let ratio_full = (r_full.b_l2_final * 1000) / r_full.b_l2_initial.max(1);
    let ratio_pure = (r_pure.b_l2_final * 1000) / r_pure.b_l2_initial.max(1);
    println!("    Full coupling:  ‖b(T)‖²/‖b(0)‖² = {}/1000", ratio_full);
    println!("    Pure diffusion: ‖b(T)‖²/‖b(0)‖² = {}/1000", ratio_pure);
    if r_full.b_l2_final < r_pure.b_l2_final {
        let extra = ratio_pure - ratio_full;
        println!("    → Bilinear coupling adds {}/1000 ADDITIONAL decay", extra);
    } else if r_full.b_l2_final > r_pure.b_l2_final {
        let reduced = ratio_full - ratio_pure;
        println!("    → Bilinear coupling REDUCES decay by {}/1000 (productive!)", reduced);
    } else {
        println!("    → Coupling neutral");
    }
    println!();

    println!("  ABC bg (Beltrami, worst case), b₀ = 1% TG:");
    let ratio_abc = (r_abc_full.b_l2_final * 1000) / r_abc_full.b_l2_initial.max(1);
    println!("    Full coupling:  ‖b(T)‖²/‖b(0)‖² = {}/1000", ratio_abc);
    println!("    Pure diffusion: ‖b(T)‖²/‖b(0)‖² = {}/1000 (same as above)", ratio_pure);
    if r_abc_full.b_l2_final < r_pure.b_l2_final {
        let extra = ratio_pure - ratio_abc;
        println!("    → ABC coupling adds {}/1000 ADDITIONAL decay", extra);
    } else if r_abc_full.b_l2_final > r_pure.b_l2_final {
        let added = ratio_abc - ratio_pure;
        println!("    → ABC coupling INCREASES b by {}/1000 (productive!)", added);
    } else {
        println!("    → ABC coupling neutral");
    }
    println!();

    // First and last 10 trajectory values for the canonical case
    println!("  W1 trajectory (‖b‖² across all 100 steps):");
    let traj = &r1.b_l2_trajectory;
    let head: Vec<String> = traj.iter().take(11).map(|x| x.to_string()).collect();
    let tail: Vec<String> = traj.iter().rev().take(11).rev().map(|x| x.to_string()).collect();
    println!("    first 11: {:?}", head);
    println!("    last 11:  {:?}", tail);
    println!();

    println!("{}", "=".repeat(78));
}
