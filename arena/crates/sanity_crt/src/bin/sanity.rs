// Compare what should be small numbers
// b_l2_sq for a small b-field
use qmnf_crtbigint::CRTBigInt;

fn main() {
    // b-field with magnitude ~100 per cell, 8³=512 cells
    // ‖b‖² ≈ 512 × 3 × 100² = 15,360,000 in i128
    let mut sum = CRTBigInt::zero();
    for _ in 0..(512*3) {
        let b = CRTBigInt::from_i64(100);
        sum = sum + b.clone() * b;
    }
    let recon = sum.garner_reconstruct();
    println!("Reconstructed sum (u128): {}", recon);
    println!("Expected i128:            15360000");
    println!("As i128:                  {}", recon as i128);
}
