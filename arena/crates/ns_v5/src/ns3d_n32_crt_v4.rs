//! # CRT-Native NS Step v4 — K-Elim Solver with Hardened Boundaries
//!
//! ## CRAM Two-Mechanism Architecture
//!
//! **Spectral (Biot-Savart) — per-cell contained 4-prime + K-Elim seed:**
//!   Each cell's vorticity is independently processed through:
//!   1. Extract 4-prime NTT residues from CRTVortExt8 state
//!   2. Forward NTT, spectral cross-product, inverse NTT — all in 4-prime basis
//!   3. Local Garner4 → raw_BS (contained, never exposed as state)
//!   4. K-Elim lift: ExtRes8::from_i128(raw_BS) — seeds p5,p6 correctly
//!   5. trunc_div_signed_8 / N^6 → velocity in ExtRes8
//!
//!   This path has NO full-field i64 reconstruction. Each cell is processed
//!   independently. raw_BS is a contained local variable, not external state.
//!
//! **Division — 6-prime K-Elim extended corridor:**
//!   trunc_div_signed_8 with DivCtx8. q_bounds 2.25M× over 4-prime.
//!
//! ## i64 Boundary Handling
//!
//!   - try_to_i64: returns Option<Vort3D> — None if any value exceeds i64
//!   - max_abs_vorticity: returns i128 — always valid
//!   - Canonical comparison disabled once i64_visible = false
//!   - CRAM state (ExtRes8) continues exactly regardless
//!
//! ## Invariants
//! A1: Zero float. D-012: Garner ascending. Recumbent: state in ExtRes8.
//! No unchecked `as i64` cast in any computation-critical path.

use crate::ns3d_n32::{N, NS_SCALE, NsParams, Vort3D, Field3D, zero_field};
use crate::ns3d_n32::{NTT_PRIMES, NTT_ROOTS, N_INV_3D, I_MOD};
use crate::ns3d::{modinv, mulmod, addmod, submod};
use crate::ns3d_n32::garner4;
use crate::cram_div_ext::{ExtRes8, DivCtx8, ext8_add, ext8_sub, ext8_mul,
                          ext8_smul, ext8_from_i64, trunc_div_signed_8, ext_to_i128_8};
use crate::assault_witness::I64BoundaryExceeded;

// ────────────────────────────────────────────────────────────────────────────
// Types
// ────────────────────────────────────────────────────────────────────────────

pub type ExtField8 = Box<[[[ExtRes8; N]; N]; N]>;

pub struct CRTVortExt8 {
    pub wx: ExtField8,
    pub wy: ExtField8,
    pub wz: ExtField8,
}

pub struct ExtVel8 { pub ux: ExtField8, pub uy: ExtField8, pub uz: ExtField8 }

fn zero_ext8() -> ExtField8 {
    Box::new(std::array::from_fn(|_|
        std::array::from_fn(|_|
            std::array::from_fn(|_| ExtRes8 { res: [0u64; 8] }))))
}

// ────────────────────────────────────────────────────────────────────────────
// CRTVortExt8 — encode/decode with boundary awareness
// ────────────────────────────────────────────────────────────────────────────

impl CRTVortExt8 {
    /// K-Elim entry: x mod pᵢ for all 8 moduli, exact.
    pub fn from_i64(w: &Vort3D) -> Self {
        let enc = |f: &Field3D| -> ExtField8 {
            Box::new(std::array::from_fn(|i|
                std::array::from_fn(|j|
                    std::array::from_fn(|k| ExtRes8::from_i128(f[i][j][k] as i128)))))
        };
        CRTVortExt8 { wx: enc(&w.wx), wy: enc(&w.wy), wz: enc(&w.wz) }
    }

    /// Maximum absolute vorticity value (i128, always valid regardless of i64 range).
    pub fn max_abs_vorticity(&self) -> i128 {
        let mut max: i128 = 0;
        for f in [&self.wx, &self.wy, &self.wz] {
            for i in 0..N { for j in 0..N { for k in 0..N {
                let v = ext_to_i128_8(&f[i][j][k]).unsigned_abs() as i128;
                if v > max { max = v; }
            }}}
        }
        max
    }

    /// True iff all vorticity values fit in i64 (abs ≤ i64::MAX).
    pub fn i64_visible(&self) -> bool {
        self.max_abs_vorticity() <= i64::MAX as i128
    }

    /// Reconstruct to i64 only if all values fit. None otherwise.
    /// Use this instead of unchecked `as i64` casts.
    pub fn try_to_i64(&self) -> Result<Vort3D, I64BoundaryExceeded> {
        let dec = |f: &ExtField8, comp: &'static str| -> Result<Field3D, I64BoundaryExceeded> {
            let mut out = [[[ 0i64; N]; N]; N];
            for i in 0..N { for j in 0..N { for k in 0..N {
                let v = ext_to_i128_8(&f[i][j][k]);
                match i64::try_from(v) {
                    Ok(x) => out[i][j][k] = x,
                    Err(_) => return Err(I64BoundaryExceeded {
                        abs_value: v.unsigned_abs() as u128,
                        step: 0,
                        component: comp,
                    }),
                }
            }}}
            Ok(out)
        };
        Ok(Vort3D {
            wx: dec(&self.wx, "wx")?,
            wy: dec(&self.wy, "wy")?,
            wz: dec(&self.wz, "wz")?,
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-cell NTT helpers — contained inside BS, never exposed
// ────────────────────────────────────────────────────────────────────────────

#[inline]
fn cfreq(k: usize) -> i64 {
    if k <= N / 2 { k as i64 } else { k as i64 - N as i64 }
}

fn ntt1d(f: &[u64; N], pi: usize) -> [u64; N] {
    let p = NTT_PRIMES[pi];
    let omega = NTT_ROOTS[pi];
    let mut powers = [0u64; N];
    powers[0] = 1;
    for idx in 1..N { powers[idx] = mulmod(powers[idx-1], omega, p); }
    let mut out = [0u64; N];
    for k in 0..N {
        let mut acc = 0u64;
        for j in 0..N {
            acc = addmod(acc, mulmod(f[j], powers[(k*j)%N], p), p);
        }
        out[k] = acc;
    }
    out
}

fn intt1d(fhat: &[u64; N], pi: usize) -> [u64; N] {
    let p = NTT_PRIMES[pi];
    let omega = NTT_ROOTS[pi];
    let mut powers = [0u64; N];
    powers[0] = 1;
    for idx in 1..N { powers[idx] = mulmod(powers[idx-1], omega, p); }
    let mut out = [0u64; N];
    for j in 0..N {
        let mut acc = 0u64;
        for k in 0..N {
            let inv_idx = (N - k) % N;
            acc = addmod(acc, mulmod(fhat[k], powers[(inv_idx*j)%N], p), p);
        }
        out[j] = acc;
    }
    out
}

fn ntt3d_4p(f: &[[[i64; N]; N]; N], pi: usize) -> Box<[[[u64; N]; N]; N]> {
    let p = NTT_PRIMES[pi];
    let mut buf: Box<[[[u64; N]; N]; N]> = Box::new([[[0u64; N]; N]; N]);
    for i in 0..N { for j in 0..N { for k in 0..N {
        buf[i][j][k] = ((f[i][j][k] % p as i64 + p as i64) as u64) % p;
    }}}
    for i in 0..N { for j in 0..N {
        let mut row = [0u64; N];
        for k in 0..N { row[k] = buf[i][j][k]; }
        let tr = ntt1d(&row, pi);
        for k in 0..N { buf[i][j][k] = tr[k]; }
    }}
    for i in 0..N { for k in 0..N {
        let mut row = [0u64; N];
        for j in 0..N { row[j] = buf[i][j][k]; }
        let tr = ntt1d(&row, pi);
        for j in 0..N { buf[i][j][k] = tr[j]; }
    }}
    for j in 0..N { for k in 0..N {
        let mut row = [0u64; N];
        for i in 0..N { row[i] = buf[i][j][k]; }
        let tr = ntt1d(&row, pi);
        for i in 0..N { buf[i][j][k] = tr[i]; }
    }}
    buf
}

fn intt3d_4p(fhat: &[[[u64; N]; N]; N], pi: usize) -> Box<[[[u64; N]; N]; N]> {
    let p = NTT_PRIMES[pi];
    let mut buf: Box<[[[u64; N]; N]; N]> = Box::new(*fhat);
    for i in 0..N { for j in 0..N {
        let mut row = [0u64; N]; for k in 0..N { row[k] = buf[i][j][k]; }
        let tr = intt1d(&row, pi);
        for k in 0..N { buf[i][j][k] = tr[k]; }
    }}
    for i in 0..N { for k in 0..N {
        let mut row = [0u64; N]; for j in 0..N { row[j] = buf[i][j][k]; }
        let tr = intt1d(&row, pi);
        for j in 0..N { buf[i][j][k] = tr[j]; }
    }}
    for j in 0..N { for k in 0..N {
        let mut row = [0u64; N]; for i in 0..N { row[i] = buf[i][j][k]; }
        let tr = intt1d(&row, pi);
        for i in 0..N { buf[i][j][k] = tr[i]; }
    }}
    let ninv = N_INV_3D[pi];
    for i in 0..N { for j in 0..N { for k in 0..N {
        buf[i][j][k] = mulmod(buf[i][j][k], ninv, p);
    }}}
    buf
}

// ────────────────────────────────────────────────────────────────────────────
// Biot-Savart — per-cell contained 4-prime + K-Elim seed
//
// Per-cell flow:
//   1. Extract 4-prime NTT residues from CRTVortExt8
//   2. Forward NTT (4 primes, 3D)
//   3. Spectral cross-product per prime
//   4. Inverse NTT (4 primes, 3D)
//   5. Local Garner4 → raw_BS (contained; never state)
//   6. K-Elim lift: from_i128(raw_BS) seeds ALL 8 lanes including p5,p6
//   7. trunc_div_signed_8 / N^6
//
// No full-field i64 reconstruction. No to_i64() call.
// raw_BS is a per-cell local variable.
// ────────────────────────────────────────────────────────────────────────────

pub struct BsRawStats {
    /// True max |raw_BS| before N^6 division (from contained Garner4)
    pub max_abs_raw_bs: i128,
    /// Division witness for d=N^6: true (raw_BS, velocity) pairs
    pub div_n6: crate::assault_witness::DivWitness,
}

pub fn biot_savart_residue_native(w: &CRTVortExt8) -> (ExtVel8, BsRawStats) {
    // Build per-prime fields — Box-allocated (heap) to avoid stack pressure
    let mut wx_f: [Box<[[[i64; N]; N]; N]>; 4] = [
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
    ];
    let mut wy_f: [Box<[[[i64; N]; N]; N]>; 4] = [
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
    ];
    let mut wz_f: [Box<[[[i64; N]; N]; N]>; 4] = [
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
        Box::new([[[0i64; N]; N]; N]), Box::new([[[0i64; N]; N]; N]),
    ];
    for pi in 0..4 {
        let p = NTT_PRIMES[pi] as i64;
        for i in 0..N { for j in 0..N { for k in 0..N {
            wx_f[pi][i][j][k] = w.wx[i][j][k].res[pi] as i64 % p;
            wy_f[pi][i][j][k] = w.wy[i][j][k].res[pi] as i64 % p;
            wz_f[pi][i][j][k] = w.wz[i][j][k].res[pi] as i64 % p;
        }}}
    }

    // Forward NTT × 4 primes
    let mut wx_hats = [
        ntt3d_4p(&wx_f[0], 0), ntt3d_4p(&wx_f[1], 1),
        ntt3d_4p(&wx_f[2], 2), ntt3d_4p(&wx_f[3], 3),
    ];
    let mut wy_hats = [
        ntt3d_4p(&wy_f[0], 0), ntt3d_4p(&wy_f[1], 1),
        ntt3d_4p(&wy_f[2], 2), ntt3d_4p(&wy_f[3], 3),
    ];
    let mut wz_hats = [
        ntt3d_4p(&wz_f[0], 0), ntt3d_4p(&wz_f[1], 1),
        ntt3d_4p(&wz_f[2], 2), ntt3d_4p(&wz_f[3], 3),
    ];

    // Spectral cross-product per prime
    let mut ux_hats: [Box<[[[u64; N]; N]; N]>; 4] = [
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
    ];
    let mut uy_hats: [Box<[[[u64; N]; N]; N]>; 4] = [
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
    ];
    let mut uz_hats: [Box<[[[u64; N]; N]; N]>; 4] = [
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
        Box::new([[[0u64; N]; N]; N]), Box::new([[[0u64; N]; N]; N]),
    ];

    for pi in 0..4 {
        let p = NTT_PRIMES[pi];
        let i_p = I_MOD[pi];
        let s = |kv: i64, x: u64| -> u64 {
            if kv >= 0 { mulmod((kv as u64) % p, x, p) }
            else { let v = mulmod(((-kv) as u64) % p, x, p); submod(0, v, p) }
        };
        for i0 in 0..N { for i1 in 0..N { for i2 in 0..N {
            let kx = cfreq(i0); let ky = cfreq(i1); let kz = cfreq(i2);
            let ksq = (kx*kx + ky*ky + kz*kz) as u64;
            if ksq == 0 { continue; }
            let inv_ksq = modinv(ksq % p, p);
            let cx = submod(
                s(ky, mulmod(wz_hats[pi][i0][i1][i2], inv_ksq, p)),
                s(kz, mulmod(wy_hats[pi][i0][i1][i2], inv_ksq, p)), p);
            let cy = submod(
                s(kz, mulmod(wx_hats[pi][i0][i1][i2], inv_ksq, p)),
                s(kx, mulmod(wz_hats[pi][i0][i1][i2], inv_ksq, p)), p);
            let cz = submod(
                s(kx, mulmod(wy_hats[pi][i0][i1][i2], inv_ksq, p)),
                s(ky, mulmod(wx_hats[pi][i0][i1][i2], inv_ksq, p)), p);
            ux_hats[pi][i0][i1][i2] = mulmod(i_p, cx, p);
            uy_hats[pi][i0][i1][i2] = mulmod(i_p, cy, p);
            uz_hats[pi][i0][i1][i2] = mulmod(i_p, cz, p);
        }}}
    }

    // Inverse NTT × 4 primes
    let ux_rs = [
        intt3d_4p(&ux_hats[0], 0), intt3d_4p(&ux_hats[1], 1),
        intt3d_4p(&ux_hats[2], 2), intt3d_4p(&ux_hats[3], 3),
    ];
    let uy_rs = [
        intt3d_4p(&uy_hats[0], 0), intt3d_4p(&uy_hats[1], 1),
        intt3d_4p(&uy_hats[2], 2), intt3d_4p(&uy_hats[3], 3),
    ];
    let uz_rs = [
        intt3d_4p(&uz_hats[0], 0), intt3d_4p(&uz_hats[1], 1),
        intt3d_4p(&uz_hats[2], 2), intt3d_4p(&uz_hats[3], 3),
    ];

    // Per-cell: local Garner4 → raw_BS → K-Elim lift → divide by N^6
    let n6: u64 = (N as u64).pow(6);
    let ctx_n6 = DivCtx8::build(n6);
    let mut ux = zero_ext8();
    let mut uy = zero_ext8();
    let mut uz = zero_ext8();
    let mut max_abs_raw_bs: i128 = 0;
    let mut div_n6_wit = crate::assault_witness::DivWitness::new("d=N^6", n6, ctx_n6.q_bound);

    for i in 0..N { for j in 0..N { for k in 0..N {
        // Contained local Garner4 — NOT exposed as state
        let rx: [u64; 4] = std::array::from_fn(|pi| ux_rs[pi][i][j][k]);
        let ry: [u64; 4] = std::array::from_fn(|pi| uy_rs[pi][i][j][k]);
        let rz: [u64; 4] = std::array::from_fn(|pi| uz_rs[pi][i][j][k]);

        let raw_x = garner4(rx) as i128;
        let raw_y = garner4(ry) as i128;
        let raw_z = garner4(rz) as i128;

        // Track true raw_BS (pre-division)
        for &rv in &[raw_x.unsigned_abs() as i128, raw_y.unsigned_abs() as i128,
                     raw_z.unsigned_abs() as i128] {
            if rv > max_abs_raw_bs { max_abs_raw_bs = rv; }
        }

        // K-Elim lift: from_i128 seeds p5=1409, p6=1601 correctly
        let ux_raw = ExtRes8::from_i128(raw_x);
        let uy_raw = ExtRes8::from_i128(raw_y);
        let uz_raw = ExtRes8::from_i128(raw_z);

        // Witnessed N^6 division: records true (raw_BS, velocity) pairs
        ux[i][j][k] = crate::cram_div_ext::trunc_div_signed_8_witnessed(&ux_raw, &ctx_n6, &mut div_n6_wit);
        uy[i][j][k] = crate::cram_div_ext::trunc_div_signed_8_witnessed(&uy_raw, &ctx_n6, &mut div_n6_wit);
        uz[i][j][k] = crate::cram_div_ext::trunc_div_signed_8_witnessed(&uz_raw, &ctx_n6, &mut div_n6_wit);
    }}}

    (ExtVel8 { ux, uy, uz }, BsRawStats { max_abs_raw_bs, div_n6: div_n6_wit })
}

// ────────────────────────────────────────────────────────────────────────────
// ns_step_crt_v4 — 6-prime extended K-Elim division NS step
// ────────────────────────────────────────────────────────────────────────────

/// Division witnesses collected during one NS step advance (for all 4 step-body sites).
pub struct StepDivWitnesses {
    pub div_s2: crate::assault_witness::DivWitness,  // diffusion: (ν·Δω) / S²
    pub div_4s: crate::assault_witness::DivWitness,  // advection: raw_adv·N / (4S)
    pub div_2s: crate::assault_witness::DivWitness,  // stretching: raw_str·N / (2S)
    pub div_n2: crate::assault_witness::DivWitness,  // update: dt·rhs / N²
}

/// NS step returning state, velocity, bs_stats, and all 5 division witnesses.
pub fn ns_step_crt_v4_with_vel(w: &CRTVortExt8, params: &NsParams)
    -> (CRTVortExt8, ExtVel8, BsRawStats, StepDivWitnesses)
{
    let (vel, bs_stats) = biot_savart_residue_native(w);

    let nu  = ext8_from_i64(params.nu_scaled);
    let dt  = ext8_from_i64(params.dt_latt);
    let n32 = ext8_from_i64(N as i64);

    let ctx_s2 = DivCtx8::build((NS_SCALE as u64).pow(2));
    let ctx_4s = DivCtx8::build(4 * NS_SCALE as u64);
    let ctx_2s = DivCtx8::build(2 * NS_SCALE as u64);
    let ctx_n2 = DivCtx8::build((N as u64).pow(2));

    // Division witnesses accumulate across all three component passes
    let mut wit_s2 = crate::assault_witness::DivWitness::new("d=S^2", (NS_SCALE as u64).pow(2), ctx_s2.q_bound);
    let mut wit_4s = crate::assault_witness::DivWitness::new("d=4S",  4 * NS_SCALE as u64,       ctx_4s.q_bound);
    let mut wit_2s = crate::assault_witness::DivWitness::new("d=2S",  2 * NS_SCALE as u64,       ctx_2s.q_bound);
    let mut wit_n2 = crate::assault_witness::DivWitness::new("d=N^2", (N as u64).pow(2),         ctx_n2.q_bound);

    // step_comp advances one vorticity component, accumulating division witnesses
    let mut step_comp_witnessed = |
        wl: &ExtField8,
        ul: &ExtField8,
        ux: &ExtField8, uy: &ExtField8, uz: &ExtField8,
        wx_f: &ExtField8, wy_f: &ExtField8, wz_f: &ExtField8,
        w_s2: &mut crate::assault_witness::DivWitness,
        w_4s: &mut crate::assault_witness::DivWitness,
        w_2s: &mut crate::assault_witness::DivWitness,
        w_n2: &mut crate::assault_witness::DivWitness,
    | -> ExtField8 {
        let mut out = zero_ext8();
        for i in 0..N {
            let ip = (i+1)%N; let im = (i+N-1)%N;
            for j in 0..N {
                let jp = (j+1)%N; let jm = (j+N-1)%N;
                for k in 0..N {
                    let kp = (k+1)%N; let km = (k+N-1)%N;

                    let lap = ext8_sub(
                        &ext8_add(
                            &ext8_add(&ext8_add(&wl[ip][j][k], &wl[im][j][k]),
                                      &ext8_add(&wl[i][jp][k], &wl[i][jm][k])),
                            &ext8_add(&wl[i][j][kp], &wl[i][j][km])),
                        &ext8_smul(&wl[i][j][k], 6));
                    let diff = crate::cram_div_ext::trunc_div_signed_8_witnessed(
                        &ext8_mul(&nu, &lap), &ctx_s2, w_s2);

                    let uv = [ux, uy, uz];
                    let sp = [(ip,j,k),(i,jp,k),(i,j,kp)];
                    let sm = [(im,j,k),(i,jm,k),(i,j,km)];
                    let mut raw_adv = ExtRes8 { res: [0u64; 8] };
                    for d in 0..3 {
                        let (s0,s1,s2) = sp[d]; let (m0,m1,m2) = sm[d];
                        let dw = ext8_sub(&wl[s0][s1][s2], &wl[m0][m1][m2]);
                        let conv = ext8_mul(&uv[d][i][j][k], &dw);
                        let cons = ext8_sub(
                            &ext8_mul(&uv[d][s0][s1][s2], &wl[s0][s1][s2]),
                            &ext8_mul(&uv[d][m0][m1][m2], &wl[m0][m1][m2]));
                        raw_adv = ext8_add(&raw_adv, &ext8_add(&conv, &cons));
                    }
                    let adv = crate::cram_div_ext::trunc_div_signed_8_witnessed(
                        &ext8_mul(&raw_adv, &n32), &ctx_4s, w_4s);

                    let dul_x = ext8_sub(&ul[ip][j][k], &ul[im][j][k]);
                    let dul_y = ext8_sub(&ul[i][jp][k], &ul[i][jm][k]);
                    let dul_z = ext8_sub(&ul[i][j][kp], &ul[i][j][km]);
                    let raw_str = ext8_add(
                        &ext8_add(&ext8_mul(&wx_f[i][j][k], &dul_x),
                                  &ext8_mul(&wy_f[i][j][k], &dul_y)),
                        &ext8_mul(&wz_f[i][j][k], &dul_z));
                    let stretch = crate::cram_div_ext::trunc_div_signed_8_witnessed(
                        &ext8_mul(&raw_str, &n32), &ctx_2s, w_2s);

                    let rhs = ext8_add(&ext8_sub(&diff, &adv), &stretch);
                    let delta = crate::cram_div_ext::trunc_div_signed_8_witnessed(
                        &ext8_mul(&dt, &rhs), &ctx_n2, w_n2);
                    out[i][j][k] = ext8_add(&wl[i][j][k], &delta);
                }
            }
        }
        out
    };

    let wx_new = step_comp_witnessed(&w.wx, &vel.ux, &vel.ux, &vel.uy, &vel.uz,
                                     &w.wx, &w.wy, &w.wz,
                                     &mut wit_s2, &mut wit_4s, &mut wit_2s, &mut wit_n2);
    let wy_new = step_comp_witnessed(&w.wy, &vel.uy, &vel.ux, &vel.uy, &vel.uz,
                                     &w.wx, &w.wy, &w.wz,
                                     &mut wit_s2, &mut wit_4s, &mut wit_2s, &mut wit_n2);
    let wz_new = step_comp_witnessed(&w.wz, &vel.uz, &vel.ux, &vel.uy, &vel.uz,
                                     &w.wx, &w.wy, &w.wz,
                                     &mut wit_s2, &mut wit_4s, &mut wit_2s, &mut wit_n2);

    let step_div_wit = StepDivWitnesses {
        div_s2: wit_s2, div_4s: wit_4s, div_2s: wit_2s, div_n2: wit_n2,
    };
    (CRTVortExt8 { wx: wx_new, wy: wy_new, wz: wz_new }, vel, bs_stats, step_div_wit)
}

/// NS step (simple interface — discards vel, bs_stats, and witnesses).
pub fn ns_step_crt_v4(w: &CRTVortExt8, params: &NsParams) -> CRTVortExt8 {
    ns_step_crt_v4_with_vel(w, params).0
}

// ────────────────────────────────────────────────────────────────────────────
// Tests — equivalence gate
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ns3d_n32::{ns_step as canon, Vort3D};

    fn diff(v4: &CRTVortExt8, wc: &Vort3D) -> (usize, i64) {
        let r = v4.try_to_i64().expect("should be i64-visible in test");
        let mut mm = 0; let mut mx = 0i64;
        for i in 0..N { for j in 0..N { for k in 0..N {
            for (a,b) in [(wc.wx[i][j][k],r.wx[i][j][k]),
                          (wc.wy[i][j][k],r.wy[i][j][k]),
                          (wc.wz[i][j][k],r.wz[i][j][k])] {
                if a!=b { mm+=1; let d=(a-b).abs(); if d>mx{mx=d;} }
            }
        }}}
        (mm, mx)
    }

    #[test]
    fn v4_step_matches_canonical_tg_100() {
        std::thread::Builder::new().stack_size(256*1024*1024).spawn(|| {
            let w0 = Vort3D::taylor_green(100);
            let p  = NsParams::cfl_max(NS_SCALE*NS_SCALE/25);
            let wc = canon(&w0, &p);
            let wv = ns_step_crt_v4(&CRTVortExt8::from_i64(&w0), &p);
            let (mm,mx) = diff(&wv, &wc);
            println!("TG100 step1: {} mismatches max_diff={}", mm, mx);
            assert_eq!(mm, 0);
        }).unwrap().join().unwrap();
    }

    #[test]
    fn v4_multistep_tg_100() {
        std::thread::Builder::new().stack_size(256*1024*1024).spawn(|| {
            let mut wc = Vort3D::taylor_green(100);
            let p = NsParams::cfl_max(NS_SCALE*NS_SCALE/25);
            let mut wv = CRTVortExt8::from_i64(&wc);
            for s in 1..=5 {
                wc = canon(&wc, &p);
                wv = ns_step_crt_v4(&wv, &p);
                let (mm,mx) = diff(&wv, &wc);
                println!("TG100 step{}: {} mismatches max_diff={}", s, mm, mx);
                assert_eq!(mm, 0, "diverges at step {}", s);
            }
        }).unwrap().join().unwrap();
    }

    #[test]
    fn v4_step_matches_canonical_abc_100() {
        std::thread::Builder::new().stack_size(256*1024*1024).spawn(|| {
            let w0 = Vort3D::abc_flow(100);
            let p  = NsParams::cfl_max(NS_SCALE*NS_SCALE/25);
            let wc = canon(&w0, &p);
            let wv = ns_step_crt_v4(&CRTVortExt8::from_i64(&w0), &p);
            let (mm,mx) = diff(&wv, &wc);
            println!("ABC100 step1: {} mismatches max_diff={}", mm, mx);
            assert_eq!(mm, 0);
        }).unwrap().join().unwrap();
    }

    #[test]
    fn v4_multistep_abc_1000() {
        std::thread::Builder::new().stack_size(256*1024*1024).spawn(|| {
            let mut wc = Vort3D::abc_flow(1000);
            let p = NsParams::cfl_max(NS_SCALE*NS_SCALE/25);
            let mut wv = CRTVortExt8::from_i64(&wc);
            for s in 1..=3 {
                wc = canon(&wc, &p);
                wv = ns_step_crt_v4(&wv, &p);
                let (mm,mx) = diff(&wv, &wc);
                println!("ABC1000 step{}: {} mismatches max_diff={}", s, mm, mx);
                assert_eq!(mm, 0, "diverges at step {}", s);
            }
        }).unwrap().join().unwrap();
    }

    /// i64 boundary detection works correctly
    #[test]
    fn i64_visibility_boundary() {
        std::thread::Builder::new().stack_size(64*1024*1024).spawn(|| {
            let w_small = Vort3D::taylor_green(100);
            let crt = CRTVortExt8::from_i64(&w_small);
            assert!(crt.i64_visible(), "small IC should be i64_visible");
            assert!(crt.try_to_i64().is_ok(), "small IC should convert to i64");
        }).unwrap().join().unwrap();
    }

    /// try_to_i64 returns error for out-of-range value
    #[test]
    fn try_to_i64_detects_overflow() {
        std::thread::Builder::new().stack_size(64*1024*1024).spawn(|| {
            let big_val = (i64::MAX as i128) + 1;
            let er8 = ExtRes8::from_i128(big_val);
            let mut out = zero_ext8();
            out[0][0][0] = er8;
            let crt = CRTVortExt8 {
                wx: out,
                wy: zero_ext8(),
                wz: zero_ext8(),
            };
            assert!(!crt.i64_visible());
            assert!(crt.try_to_i64().is_err());
        }).unwrap().join().unwrap();
    }
}

    /// P5B-8G: Full 20-step hardened assault — witness protocol, no float
    #[test]
    fn p5b8g_hardened_assault_20_steps() {
        use crate::assault_witness::{ExactRatio, CorridorWitness, DivWitness};
        use crate::ns3d_n32::ns_step as canon_step;

        std::thread::Builder::new().stack_size(256*1024*1024).spawn(|| {
            let ns = NS_SCALE as u64;
            let params = NsParams::cfl_max(NS_SCALE*NS_SCALE/25);
            let m_ntt4_half: u128 = (769u128*929*1153*1217)/2;
            let m_ntt6_half: u128 = crate::ns3d_n32_ntt6::M_NTT_6 / 2;

            let ctx_n6 = DivCtx8::build((N as u64).pow(6));

            println!("\n=== P5B-8G Hardened 20-Step Assault ===");
            println!("M_NTT_4/2: {} | M_NTT_6/2: {}", m_ntt4_half, m_ntt6_half);

            let w0 = Vort3D::abc_flow(NS_SCALE);
            let mut w_crt = CRTVortExt8::from_i64(&w0);
            let mut w_canon = w0;
            let mut canon_alive = true;
            let mut all_ok = true;

            for step in 1..=20usize {
                let (w_next, vel, bs, step_div) = ns_step_crt_v4_with_vel(&w_crt, &params);
                let raw_bs = bs.max_abs_raw_bs;
                let wort   = w_next.max_abs_vorticity();
                let i64vis  = wort <= i64::MAX as i128;

                // K-Elim roundtrip check
                let mut kelim_ok = true;
                for i in 0..N { for j in 0..N { for k in 0..N {
                    for c in [&vel.ux,&vel.uy,&vel.uz] {
                        let v = ext_to_i128_8(&c[i][j][k]);
                        if ext_to_i128_8(&ExtRes8::from_i128(v)) != v { kelim_ok = false; }
                    }
                }}}

                let ntt4_ok = raw_bs as u128 <= m_ntt4_half;
                let ntt6_ok = raw_bs as u128 <= m_ntt6_half;
                let n6_ok   = bs.div_n6.contract_ok;
                let step_ok = kelim_ok && n6_ok && ntt4_ok && ntt6_ok &&
                              step_div.div_s2.contract_ok && step_div.div_4s.contract_ok &&
                              step_div.div_2s.contract_ok && step_div.div_n2.contract_ok;
                if !step_ok { all_ok = false; }

                // Canonical match check (post-advance state)
                let canon_match = if canon_alive && i64vis {
                    match w_next.try_to_i64() {
                        Ok(wv) => {
                            let wc = canon_step(&w_canon, &params);
                            let matched = !(0..N).any(|i| (0..N).any(|j| (0..N).any(|k|
                                wc.wx[i][j][k]!=wv.wx[i][j][k] ||
                                wc.wy[i][j][k]!=wv.wy[i][j][k] ||
                                wc.wz[i][j][k]!=wv.wz[i][j][k]
                            )));
                            if matched { w_canon = wc; } else { canon_alive = false; }
                            matched
                        }
                        Err(_) => { canon_alive = false; false }
                    }
                } else { false };

                // P5B-8C: integer-only percent
                let pct4 = ExactRatio::new(raw_bs as u128, m_ntt4_half);
                let pct6 = ExactRatio::new(raw_bs as u128, m_ntt6_half);

                println!("step {:>3}: wort={:>22} raw_bs={:>15} pct4={} pct6={} i64={} canon={} ok={}",
                    step, wort, raw_bs,
                    pct4.format_pct_2dp(), pct6.format_pct_10dp(),
                    if i64vis {"vis"} else {"BIG"},
                    if canon_match {"ALIVE"} else {" DEAD"},
                    if step_ok {"✓"} else {"WARN"});

                if !step_ok {
                    println!("  ABORT step={}", step);
                    break;
                }

                w_crt = w_next;
            }

            println!("All witnesses clean: {}", all_ok);
            assert!(all_ok, "P5B-8G: witness violation detected");
        }).unwrap().join().unwrap();
    }
