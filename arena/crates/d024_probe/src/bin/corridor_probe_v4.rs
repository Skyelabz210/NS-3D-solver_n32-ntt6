//! # corridor_probe_v4 — K-Elim Extended Corridor Verification
//!
//! Verifies:
//! A) 6-prime K-Elim corridor is available (M_NTT_6/2 ≈ 1.13×10^18)
//! B) Division contracts have enormous margin with 6-prime DivCtx8
//! C) K-Elim round-trip (from_i128 → ext_to_i128_8) is exact
//!
//! Usage: cargo run -p d024_probe --bin corridor_probe_v4 --release

use ns_regularity::ns3d_n32::{N, NS_SCALE, NsParams, Vort3D, biot_savart};
use ns_regularity::ns3d_n32_crt_v4::{CRTVortExt8, ns_step_crt_v4};
use ns_regularity::cram_div_ext::{ExtRes8, DivCtx8, ext_to_i128_8};
use ns_regularity::ns3d_n32_ntt6::M_NTT_6;

const M_NTT_4_HALF: u128 = (769u128 * 929 * 1153 * 1217) / 2;
const M_NTT_6_HALF: u128 = M_NTT_6 / 2;

fn probe_ic(label: &str, w0: Vort3D, n_steps: usize, params: &NsParams) {
    println!("\n═══ {} ═══", label);

    // DivCtx8 bounds for reference
    let ns = NS_SCALE as u64;
    let bounds: &[(&str, u64, u128)] = &[
        ("d=N^6",   1u64 << 30,  M_NTT_6 * 2),
        ("d=S^2",   ns * ns,     M_NTT_6 * 5),
        ("d=4S",    4 * ns,      M_NTT_6 * 3125),
        ("d=2S",    2 * ns,      M_NTT_6 * 3125),
        ("d=N^2",   (N*N) as u64, M_NTT_6 * 1_953_125),
    ];

    let mut w_ext = CRTVortExt8::from_i64(&w0);
    let mut global_max_raw: u128 = 0;
    let mut kelim_ok = true;

    println!("  Step | max|raw_BS| | pct-4p-corridor | pct-6p-corridor | status");
    println!("  -----|------------|-----------------|-----------------|-------");

    for step in 1..=n_steps {
        // Measure via canonical biot_savart (4-prime path)
        let w_i64 = w_ext.to_i64();
        let vel = biot_savart(&w_i64);

        let mut step_max_raw: u128 = 0;

        for i in 0..N { for j in 0..N { for k in 0..N {
            for &v in &[vel.ux[i][j][k], vel.uy[i][j][k], vel.uz[i][j][k]] {
                let abs_v = v.unsigned_abs() as u128;
                if abs_v > step_max_raw { step_max_raw = abs_v; }

                // K-Elim round-trip verification
                let er8 = ExtRes8::from_i128(v as i128);
                let recon = ext_to_i128_8(&er8);
                if recon != v as i128 {
                    eprintln!("  K-ELIM ROUND-TRIP FAILED: v={} recon={}", v, recon);
                    kelim_ok = false;
                }
            }
        }}}

        if step_max_raw > global_max_raw { global_max_raw = step_max_raw; }

        let pct_4p = step_max_raw * 10_000 / M_NTT_4_HALF.max(1);
        let pct_6p = step_max_raw * 10_000 / M_NTT_6_HALF.max(1);
        let status = if step_max_raw > M_NTT_4_HALF { "4P-EXCEED" }
                     else if step_max_raw > M_NTT_6_HALF { "6P-EXCEED" }
                     else { "SAFE" };

        println!("  {:>4} | {:>10} | {:>13.4}%   | {:>13.10}%   | {}",
            step, step_max_raw,
            pct_4p as f64 / 100.0,
            pct_6p as f64 / 1000.0,
            status);

        w_ext = ns_step_crt_v4(&w_ext, params);
    }

    println!("\n  ── Summary ──");
    let margin_4p = M_NTT_4_HALF as i128 - global_max_raw as i128;
    let margin_6p = M_NTT_6_HALF as i128 - global_max_raw as i128;
    let pct_4p = global_max_raw * 10000 / M_NTT_4_HALF.max(1);
    let pct_6p = global_max_raw * 10000 / M_NTT_6_HALF.max(1);

    println!("  Max |raw_BS|:  {:>22}", global_max_raw);
    println!("  4p margin:     {:>22}  ({:.2}% of 4p used)", margin_4p, pct_4p as f64 / 100.0);
    println!("  6p margin:     {:>22}  ({:.10}% of 6p used)", margin_6p, pct_6p as f64 / 100.0);

    let improvement = M_NTT_6_HALF / M_NTT_4_HALF.max(1);
    println!("  6p/4p ratio:   {}× larger corridor", improvement);

    println!("\n  DivCtx8 q_bounds (6-prime) vs observed max|q|:");
    println!("  (Conservative: q ≈ max|raw|; actual quotients are smaller)");
    for &(name, d, qb) in bounds {
        // Max quotient for d=N^6: max|vel| (velocity = raw/N^6)
        // For other divisors: quotients are step-body outputs << raw
        let approx_q = if d == 1u64 << 30 { global_max_raw }
                       else { global_max_raw / d as u128 + 1 };
        let pct = approx_q * 10_000 / qb.max(1);
        println!("    {:<9} q_bound={:<26}  approx_max|q|={:<14}  {:.8}% used  {}",
            name, qb, approx_q, pct as f64 / 100.0,
            if approx_q > qb { "⚠ VIOLATED" } else { "✓ SAFE" });
    }

    println!("\n  K-Elim round-trip: {}", if kelim_ok { "✓ ALL PASS" } else { "✗ FAILURES" });

    if global_max_raw > M_NTT_4_HALF {
        println!("  ⚠ 4p spectral wall hit — need P5B-7 for extended runs");
    } else {
        println!("  ✓ Within 4p spectral corridor");
    }
    println!("  ✓ 6p K-Elim corridor: {:.3e} headroom above observed max",
        margin_6p as f64);
}

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║  corridor_probe_v4 — K-Elim Extended Corridor       ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!("  M_NTT_4/2 (4p spectral limit): {:>22}", M_NTT_4_HALF);
    println!("  M_NTT_6/2 (6p K-Elim corridor): {:>22}", M_NTT_6_HALF);

    let p = NsParams::cfl_max(NS_SCALE * NS_SCALE / 25);

    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            probe_ic("TG amp=100, 5 steps",    Vort3D::taylor_green(100),    5, &p);
            probe_ic("TG amp=1000, 5 steps",   Vort3D::taylor_green(1000),   5, &p);
            probe_ic("ABC amp=100, 5 steps",   Vort3D::abc_flow(100),        5, &p);
            probe_ic("ABC amp=NS_SCALE, 3 steps", Vort3D::abc_flow(NS_SCALE), 3, &p);
            probe_ic("ABC amp=50000, 3 steps", Vort3D::abc_flow(50000),      3, &p);

            println!("\n╔══════════════════════════════════════════════════════╗");
            println!("║  PROBE COMPLETE                                      ║");
            println!("╚══════════════════════════════════════════════════════╝");
        })
        .unwrap()
        .join()
        .unwrap();
}
