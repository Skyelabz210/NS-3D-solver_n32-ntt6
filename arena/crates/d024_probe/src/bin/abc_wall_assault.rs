//! # ABC Wall Assault — P5B-8H.1/2/3 (All Reviewer Corrections Applied)
//!
//! Corrections from executive review:
//!   H.1: All 5 division sites instrumented (N^6 from BS, S^2/4S/2S/N^2 from step body)
//!        True dividends from BS: raw_BS before division, not q*N^6
//!   H.2: ExtRes8 corridor = M_NTT_6 × 2^31 × 5^9 / 2 (correct, checked arithmetic)
//!        Abort on ntt4_corridor.exceeded (hard stop under 4p BS architecture)
//!        NTT4_THIN_MARGIN warning at 10^7
//!   H.3: Step indexing fixed: operator witness (pre-advance), state witness (post-advance)
//!        canonical_alive updated from actual mismatch, not only from i64 failure

use std::io::{self, Write};
use ns_regularity::ns3d_n32::{N, NS_SCALE, NsParams, Vort3D};
use ns_regularity::ns3d_n32_crt_v4::{CRTVortExt8, ns_step_crt_v4_with_vel, ExtVel8, BsRawStats, StepDivWitnesses};
use ns_regularity::cram_div_ext::{ExtRes8, DivCtx8, ext_to_i128_8};
use ns_regularity::ns3d_n32_ntt6::M_NTT_6;
use ns_regularity::assault_witness::{
    ExactRatio, CorridorWitness, DivWitness, OperatorWitness, StateWitness,
};
use ns_regularity::ns3d_n32::ns_step as canon_step;

// ── Constants (checked arithmetic required for proof corridors) ───────────────

const M_NTT_4_HALF: u128 = (769u128 * 929 * 1153 * 1217) / 2;  // 501_224_098_800
const M_NTT_6_HALF: u128 = M_NTT_6 / 2;

/// ExtRes8 corridor half-width: M_NTT_6 × 2^31 × 5^9 / 2 (reviewer error 7)
const MOD_2POW_128: u128 = 1u128 << 31;    // 2_147_483_648
const MOD_5POW_128: u128 = 1_953_125u128;  // 5^9

fn ext8_corridor_half() -> u128 {
    M_NTT_6
        .checked_mul(MOD_2POW_128).expect("ext8 corridor product overflow at 2^31")
        .checked_mul(MOD_5POW_128).expect("ext8 corridor product overflow at 5^9")
        / 2
}

/// NTT4 thin-margin threshold: warn when closer than this to the 4p wall
const NTT4_THIN_MARGIN: u128 = 10_000_000;

const MAX_STEPS: usize = 20;

// ── OperatorWitness builder ───────────────────────────────────────────────────

fn build_operator_witness(
    step: usize,
    vel: &ExtVel8,
    bs_stats: &BsRawStats,
    step_div: &StepDivWitnesses,
) -> OperatorWitness {
    let max_abs_raw_bs = bs_stats.max_abs_raw_bs;
    let max_abs_velocity = bs_stats.div_n6.max_abs_quotient;

    // K-Elim round-trip: verify velocity cells can be re-encoded exactly
    let mut kelim_ok = true;
    for i in 0..N { for j in 0..N { for k in 0..N {
        for comp in [&vel.ux, &vel.uy, &vel.uz] {
            let v = ext_to_i128_8(&comp[i][j][k]);
            if ext_to_i128_8(&ExtRes8::from_i128(v)) != v { kelim_ok = false; }
        }
    }}}

    // Corridor witnesses (P5B-8D: raw_BS, not velocity)
    let ntt4_w = CorridorWitness::measure("NTT4/raw_BS", M_NTT_4_HALF,  max_abs_raw_bs as u128);
    let ntt6_w = CorridorWitness::measure("NTT6/K-Elim", M_NTT_6_HALF,  max_abs_raw_bs as u128);
    let thin = ntt4_w.thin_margin(NTT4_THIN_MARGIN);

    OperatorWitness {
        step,
        max_abs_raw_bs,
        max_abs_velocity,
        div_n6: bs_stats.div_n6.clone(),
        div_s2: step_div.div_s2.clone(),
        div_4s: step_div.div_4s.clone(),
        div_2s: step_div.div_2s.clone(),
        div_n2: step_div.div_n2.clone(),
        ntt4_corridor: ntt4_w,
        ntt6_corridor: ntt6_w,
        kelim_roundtrip_ok: kelim_ok,
        ntt4_thin_margin: thin,
    }
}

// ── StateWitness builder ──────────────────────────────────────────────────────

fn build_state_witness(
    state_index: usize,
    w_next: &CRTVortExt8,
    w_canon_next: Option<&Vort3D>,
    ext8_half: u128,
) -> StateWitness {
    let max_abs_vorticity = w_next.max_abs_vorticity();
    let i64_visible = max_abs_vorticity <= i64::MAX as i128;

    let canonical_alive = if let (Some(wc), true) = (w_canon_next, i64_visible) {
        match w_next.try_to_i64() {
            Ok(wv) => !(0..N).any(|i| (0..N).any(|j| (0..N).any(|k|
                wc.wx[i][j][k] != wv.wx[i][j][k] ||
                wc.wy[i][j][k] != wv.wy[i][j][k] ||
                wc.wz[i][j][k] != wv.wz[i][j][k]
            ))),
            Err(_) => false,
        }
    } else { false };

    let ext8_w = CorridorWitness::measure("ExtRes8/vort", ext8_half, max_abs_vorticity as u128);

    StateWitness { state_index, max_abs_vorticity, i64_visible, canonical_alive, extres8_corridor: ext8_w }
}

// ── Assault runner ────────────────────────────────────────────────────────────

fn run_assault() {
    let params = NsParams::cfl_max(NS_SCALE * NS_SCALE / 25);
    let ext8_h = ext8_corridor_half();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  ABC WALL ASSAULT — P5B-8H.1/2/3 (MAX_STEPS={:>3})          ║", MAX_STEPS);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("  Corrections: all 5 div sites instrumented | true dividends");
    println!("               fixed ExtRes8 corridor | ntt4 hard abort");
    println!("               step indexing: operator pre-advance, state post-advance");
    println!("               canonical_alive updated from mismatch detection");
    println!("  IC: ABC amp={}", NS_SCALE);
    println!("  NTT4/raw_BS wall (Garner4 precond): {}", M_NTT_4_HALF);
    println!("  NTT6/K-Elim corridor (division):    {}", M_NTT_6_HALF);
    println!("  ExtRes8 corridor (state):            {}", ext8_h);
    println!("  NTT4_THIN_MARGIN guard:              {}", NTT4_THIN_MARGIN);
    io::stdout().flush().unwrap();

    println!();
    println!("  Terminology:");
    println!("    4p spectral witness basis: Garner4 used for raw_BS reconstruction (HARD PRECONDITION)");
    println!("    6p division/state basis:   K-Elim used for all quotients and ExtRes8 state");
    println!();
    println!("  Op n→n+1           | raw_BS          | pct-4p     | thin | all_div | KElim");
    println!("  -------------------|-----------------|------------|------|---------|------");
    println!("  Post-state n+1     | max|vorticity|  | i64        | canon");
    println!("  -------------------|-----------------|------------|------");
    io::stdout().flush().unwrap();

    let w0 = Vort3D::abc_flow(NS_SCALE);
    let mut w_crt = CRTVortExt8::from_i64(&w0);
    let mut w_canon = w0;
    let mut canonical_alive = true;
    let mut first_i64_bnd: Option<usize> = None;
    let mut op_witnesses: Vec<OperatorWitness> = Vec::with_capacity(MAX_STEPS);
    let mut st_witnesses: Vec<StateWitness>    = Vec::with_capacity(MAX_STEPS);

    eprintln!("P5B_ASSAULT_ENTER_STEP_LOOP max_steps={}", MAX_STEPS);
    io::stderr().flush().unwrap();

    for step in 1..=MAX_STEPS {
        eprintln!("P5B_STEP_BEGIN {}", step);
        io::stderr().flush().unwrap();

        // Advance CRAM: single BS call returns state, vel, bs_stats, step_div_witnesses
        let (w_next, vel, bs_stats, step_div) = ns_step_crt_v4_with_vel(&w_crt, &params);

        // OPERATOR witness (pre-advance data: raw_BS, velocity, all 5 divisions)
        let op_wit = build_operator_witness(step, &vel, &bs_stats, &step_div);

        // Advance canonical (while alive and while w_crt is i64-visible)
        let w_canon_next = if canonical_alive {
            match w_crt.try_to_i64() {
                Ok(_) => {
                    let wc = canon_step(&w_canon, &params);
                    Some(wc)
                }
                Err(_) => {
                    canonical_alive = false;
                    None
                }
            }
        } else { None };

        // STATE witness (post-advance data: w_next vorticity + canonical match)
        let st_wit = build_state_witness(step, &w_next, w_canon_next.as_ref(), ext8_h);

        // Update outer canonical_alive from mismatch (reviewer error 6)
        if canonical_alive && !st_wit.canonical_alive {
            canonical_alive = false;
            println!("  [step {}: mismatch] canonical disabled — diverged from v4", step);
            io::stdout().flush().unwrap();
        }

        // Track i64 boundary
        if !st_wit.i64_visible && first_i64_bnd.is_none() { first_i64_bnd = Some(step); }

        // Print operator witness row
        let pct_4p  = ExactRatio::new(op_wit.max_abs_raw_bs as u128, M_NTT_4_HALF);
        let thin_tag = if op_wit.ntt4_thin_margin { "THIN" } else { "ok  " };
        let div_tag  = if op_wit.all_contracts_ok() && op_wit.all_instrumented() { "OK " } else if !op_wit.all_instrumented() { "!INST" } else { "VIOL" };
        let kel_tag  = if op_wit.kelim_roundtrip_ok { "OK" } else { "FAIL" };

        println!("  Op {:>2}→{:<2}             | {:>15} | {:>10} | {:>4} | {:>7} | {:>4}",
            step-1, step, op_wit.max_abs_raw_bs, pct_4p.format_pct_2dp(), thin_tag, div_tag, kel_tag);

        // Print state witness row
        let i64_tag  = if st_wit.i64_visible   { "vis" } else { "BIG" };
        let can_tag  = if st_wit.canonical_alive { "ALIVE" } else { " DEAD" };
        println!("  St {:<4}               | {:>22} | {:>3} | {:>5}",
            step, st_wit.max_abs_vorticity, i64_tag, can_tag);
        io::stdout().flush().unwrap();

        // Abort conditions (reviewer error 4: ntt4 is hard stop)
        if let Some(reason) = op_wit.should_abort() {
            println!("\n  ⚠ ABORT step {}: {}", step, reason);
            io::stdout().flush().unwrap();
            op_witnesses.push(op_wit);
            st_witnesses.push(st_wit);
            break;
        }

        op_witnesses.push(op_wit);
        st_witnesses.push(st_wit);

        // Advance
        if let Some(wc) = w_canon_next { w_canon = wc; }
        w_crt = w_next;

        eprintln!("P5B_STEP_DONE {}", step);
        io::stderr().flush().unwrap();

        if step == MAX_STEPS {
            println!("\n  ✓ MAX_STEPS={} reached — no violations.", MAX_STEPS);
            io::stdout().flush().unwrap();
        }
    }

    // Final report
    println!();
    println!("  ── Operator Summary (raw_BS corridor) ──");
    for op in &op_witnesses {
        let m = match &op.ntt4_corridor.margin {
            ns_regularity::assault_witness::CorridorMargin::Safe(v) => format!("+{}", v),
            ns_regularity::assault_witness::CorridorMargin::Exceeded(v) => format!("-{}", v),
        };
        println!("    op {:>2}→{:<2}: raw_BS={:<15}  4p_margin={}{}",
            op.step-1, op.step, op.max_abs_raw_bs, m,
            if op.ntt4_thin_margin { " ⚠THIN" } else { "" });
    }

    println!();
    println!("  ── State Summary (vorticity) ──");
    for (i, st) in st_witnesses.iter().enumerate() {
        let growth = if i > 0 {
            let prev = st_witnesses[i-1].max_abs_vorticity.unsigned_abs() as u128;
            let curr = st.max_abs_vorticity.unsigned_abs() as u128;
            format!("  ratio×100={}", ExactRatio::new(curr, prev.max(1)).basis_points())
        } else { String::new() };
        println!("    state {:>3}: {:>22}{}", st.state_index, st.max_abs_vorticity, growth);
    }

    println!();
    println!("  ── Division Witnesses (final step) ──");
    if let Some(op) = op_witnesses.last() {
        op.div_n6.print();
        op.div_s2.print();
        op.div_4s.print();
        op.div_2s.print();
        op.div_n2.print();
    }

    let all_ok = op_witnesses.iter().all(|op| op.should_abort().is_none());
    println!();
    if all_ok {
        println!("  ✓ {} STEPS — all witnesses clean. Zero violations.", op_witnesses.len());
        println!("  Architecture: 4p spectral basis (Garner4 precondition) +");
        println!("                6p division/state basis (K-Elim division corridor)");
        println!("  First i64 boundary step: {:?}", first_i64_bnd);
        println!("  D-024 firewall: discrete integer CRAM-NS evidence only.");
    }
    io::stdout().flush().unwrap();
}

fn main() {
    std::thread::Builder::new()
        .stack_size(256*1024*1024)
        .spawn(run_assault)
        .unwrap()
        .join()
        .unwrap();
}
