//! # Phase 1 Kill-Tests
//!
//! Per advisor on Push 8:
//! "Add kill-tests before any D-024 claim."
//!
//! These tests check claims that Push 7/8 implicitly assumed:
//! 1. CRTBigInt signed arithmetic obeys ring laws
//! 2. The CFL formula in the archived solver
//! 3. Garner reconstruction is checked, not silently wrapping
//! 4. CRT division by SCALE matches integer truncating division

use qmnf_crtbigint::CRTBigInt;
use ns_regularity::ns3d::{N, NS_SCALE};

fn main() {
    println!("PHASE 1 KILL-TESTS");
    println!("==================\n");

    println!("KT1: Signed arithmetic identity laws");
    println!("------------------------------------");

    // KT1a: 1 + (-1) == 0 ?
    let a = CRTBigInt::from_i64(1);
    let b = CRTBigInt::from_i64(-1);
    let sum = a + b;
    let recon = sum.to_i64();
    println!("  1 + (-1) = {:?}  expected: Some(0)", recon);
    println!("    PASS: {}", recon == Some(0));
    println!();

    // KT1b: 2 - 5 == -3 ?
    let a = CRTBigInt::from_i64(2);
    let b = CRTBigInt::from_i64(5);
    let diff = a - b;
    let recon = diff.to_i64();
    println!("  2 - 5 = {:?}  expected: Some(-3)", recon);
    println!("    PASS: {}", recon == Some(-3));
    println!();

    // KT1c: (-3) * 7 == -21 ?
    let a = CRTBigInt::from_i64(-3);
    let b = CRTBigInt::from_i64(7);
    let prod = a * b;
    let recon = prod.to_i64();
    println!("  (-3) * 7 = {:?}  expected: Some(-21)", recon);
    println!("    PASS: {}", recon == Some(-21));
    println!();

    // KT1d: 5 * 0 == 0 ?
    let a = CRTBigInt::from_i64(5);
    let b = CRTBigInt::from_i64(0);
    let prod = a * b;
    let recon = prod.to_i64();
    println!("  5 * 0 = {:?}  expected: Some(0)", recon);
    println!("    PASS: {}", recon == Some(0));
    println!();

    // KT1e: 0 + (-7) == -7 ?
    let a = CRTBigInt::from_i64(0);
    let b = CRTBigInt::from_i64(-7);
    let sum = a + b;
    let recon = sum.to_i64();
    println!("  0 + (-7) = {:?}  expected: Some(-7)", recon);
    println!("    PASS: {}", recon == Some(-7));
    println!();

    // KT1f: (-3) + 3 == 0 ?
    let a = CRTBigInt::from_i64(-3);
    let b = CRTBigInt::from_i64(3);
    let sum = a + b;
    let recon = sum.to_i64();
    println!("  (-3) + 3 = {:?}  expected: Some(0)", recon);
    println!("    PASS: {}", recon == Some(0));
    println!();

    println!("KT2: CFL check at Push 7/8 parameters");
    println!("--------------------------------------");
    let nu_scaled: i64 = 1_000_000;
    let dt_latt: i64 = 100;
    let left = 6 * nu_scaled * dt_latt;
    let right = (NS_SCALE as i64) * (NS_SCALE as i64) * (N * N) as i64;
    println!("  Solver CFL formula: 6 * nu * dt <= NS_SCALE^2 * N^2");
    println!("  left  = 6 * {} * {} = {}", nu_scaled, dt_latt, left);
    println!("  right = {}^2 * {} = {}", NS_SCALE, N*N, right);
    println!("  CFL passes: {}", left <= right);
    println!("  Push 8 claimed CFL violation. The archived solver's own");
    println!("  formula says dt=100 is fine. This is a contradiction.");
    println!();

    println!("KT3: CRT division vs integer truncating division");
    println!("------------------------------------------------");
    // Take a small "error" value: 5. Integer scheme: 5 / SCALE = 5/10000 = 0.
    // CRT scheme: 5 * SCALE^{-1} mod p = some random-looking residue.
    let small_val = CRTBigInt::from_i64(5);
    let scale = CRTBigInt::from_u64(NS_SCALE as u64);
    let inv_scale = scale.mod_inverse().unwrap();
    let crt_result = small_val * inv_scale;
    let crt_recon_attempt = crt_result.to_i64();
    println!("  Integer truncating: 5 / {} = 0", NS_SCALE);
    println!("  CRT modular inverse result: residue lanes:");
    for i in 0..8 {
        println!("    lane {}: {}", i, crt_result.residue(i));
    }
    println!("  CRT to_i64: {:?}", crt_recon_attempt);
    println!("  These are NOT the same operation. Modular inverse is a");
    println!("  bijection on the multiplicative group; division-with-remainder");
    println!("  truncation is not. The Push 7/8 step uses the wrong one.");
    println!();

    println!("KT4: Silent Garner wrap detection");
    println!("---------------------------------");
    // Build a value that exceeds u128 in true magnitude. Each lane is ~2^63.
    // The product of 2 lanes already exceeds 2^126. A value with all lanes
    // near max should reconstruct to something well past u128.
    let big = CRTBigInt {
        residues: [
            9_223_372_036_854_775_700,
            9_223_372_036_854_775_500,
            9_223_372_036_854_775_400,
            9_223_372_036_854_775_300,
            9_223_372_036_854_775_200,
            9_223_372_036_854_775_100,
            9_223_372_036_854_775_000,
            9_223_372_036_854_774_900,
        ],
        sign: 1,
    };
    let g = big.garner_reconstruct();
    println!("  garner_reconstruct on near-max-residue all-lane: {}", g);
    println!("  True product of moduli is ~2^504, far exceeds 2^128.");
    println!("  Yet garner_reconstruct returns a u128 without panic.");
    println!("  This is wrapping_add/wrapping_mul behavior, silent overflow.");
    println!("  Push 7/8's 'Some(u128)' diagnostics do not prove fits-in-u128.");
}
