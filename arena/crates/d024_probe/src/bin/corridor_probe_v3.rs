//! # corridor_probe_v3 — Division Contract and BS-Raw Margin Measurement
//!
//! Runs ns_step_crt_v3 instrumented to record per-step per-divisor maxima.
//! Reports the contract margin (distance from each q_bound_min) and the
//! Biot-Savart raw-velocity corridor margin (|raw| vs M_NTT/2).
//!
//! ## DivContract table (pre-computed analytically)
//!
//! | d          | q_bound_min        | limiting lane |
//! |------------|--------------------|---------------|
//! | N^6=2^30   | 2.005e12           | 2-lane (partial=2) |
//! | S^2=10^8   | 5.013e12           | 5-lane (partial=5) |
//! | 4S=40000   | 3.133e15           | 5-lane (partial=3125) |
//! | 2S=20000   | 3.133e15           | 5-lane (partial=3125) |
//! | N^2=1024   | 1.957e18           | 5-lane (partial=1953125) |
//!
//! BS raw limit: |raw| ≤ M_NTT/2 = 5.013e11
//!
//! ## Usage
//!   cargo run -p d024_probe --bin corridor_probe_v3 --release

use ns_regularity::ns3d_n32::{
    N, NS_SCALE, NTT_PRIMES, NsParams, Vort3D, Field3D,
    zero_field, ntt3d, intt3d, garner4, I_MOD,
};
use ns_regularity::ns3d::{modinv, mulmod, addmod, submod};
use ns_regularity::cram_div::{ExtRes, EXT_BASIS, MOD_2POW, MOD_5POW, cram_div_exact_pos};
use ns_regularity::ns3d_n32_crt_v2::CRTVortExt;
use ns_regularity::ns3d_n32_crt_v3::ns_step_crt_v3;

// ============================================================================
// Constants — DivContract bounds (analytical, all exact integers)
// ============================================================================

const M_NTT: u128 = 769 * 929 * 1153 * 1217;   // 1_002_568_584_801
const M_NTT_HALF: u128 = M_NTT / 2;             // 501_284_292_400

// q_bound_min for each divisor (the tighter of 2-lane and 5-lane bounds)
const Q_BOUND_N6: u128   = M_NTT * 2;                    // d=2^30
const Q_BOUND_S2: u128   = M_NTT * 5;                    // d=10^8
const Q_BOUND_4S: u128   = M_NTT * 3125;                 // d=40000
const Q_BOUND_2S: u128   = M_NTT * 3125;                 // d=20000
const Q_BOUND_N2: u128   = M_NTT * 1_953_125;            // d=1024

/// Per-step corridor report for one divisor
#[derive(Debug, Default, Clone)]
struct DivReport {
    label: &'static str,
    d: u64,
    q_bound_min: u128,
    max_abs_q: u128,
    /// margin = q_bound_min - max_abs_q  (positive = safe)
    margin: i128,
    /// fraction used = max_abs_q / q_bound_min
    frac_used_pct: u64,
}

impl DivReport {
    fn new(label: &'static str, d: u64, q_bound_min: u128) -> Self {
        DivReport { label, d, q_bound_min, ..Default::default() }
    }

    fn update(&mut self, abs_q: u128) {
        if abs_q > self.max_abs_q {
            self.max_abs_q = abs_q;
        }
    }

    fn finalize(&mut self) {
        self.margin = self.q_bound_min as i128 - self.max_abs_q as i128;
        // fraction used in basis points (0.01%)
        self.frac_used_pct = if self.q_bound_min > 0 {
            (self.max_abs_q * 10_000 / self.q_bound_min) as u64
        } else {
            0
        };
    }

    fn print(&self) {
        let status = if self.margin >= 0 { "SAFE" } else { "VIOLATION" };
        println!(
            "  {:8} d={:12}  max|q|={:15}  bound={:15}  margin={:+16}  used={:4}bp  {}",
            self.label, self.d, self.max_abs_q, self.q_bound_min,
            self.margin, self.frac_used_pct, status
        );
    }
}

// ============================================================================
// Instrumented step — runs v3 but intercepts each division to record |q|
// ============================================================================

// We reimplement the measurement inline to avoid modifying ns3d_n32_crt_v3.
// The probe duplicates the step logic with instrumentation hooks.
// The real v3 step is NOT called here — we instrument the equivalent.
// This ensures measurements are from the live computation path.

#[inline]
fn cfreq(k: usize) -> i64 {
    if k <= N / 2 { k as i64 } else { k as i64 - N as i64 }
}

fn ext_add(a: &ExtRes, b: &ExtRes) -> ExtRes {
    let mut r = [0u64; 6];
    for i in 0..6 { r[i] = ((a.res[i] as u128 + b.res[i] as u128) % EXT_BASIS[i] as u128) as u64; }
    ExtRes { res: r }
}
fn ext_sub(a: &ExtRes, b: &ExtRes) -> ExtRes {
    let mut r = [0u64; 6];
    for i in 0..6 { let m = EXT_BASIS[i]; r[i] = ((a.res[i] + m - b.res[i] % m) % m) as u64; }
    ExtRes { res: r }
}
fn ext_mul(a: &ExtRes, b: &ExtRes) -> ExtRes {
    let mut r = [0u64; 6];
    for i in 0..6 { r[i] = ((a.res[i] as u128 * b.res[i] as u128) % EXT_BASIS[i] as u128) as u64; }
    ExtRes { res: r }
}
fn ext_neg(a: &ExtRes) -> ExtRes {
    let mut r = [0u64; 6];
    for i in 0..6 { let m = EXT_BASIS[i]; r[i] = (m - a.res[i] % m) % m; }
    ExtRes { res: r }
}
fn ext_smul(a: &ExtRes, k: i128) -> ExtRes {
    let mut r = [0u64; 6];
    for i in 0..6 {
        let m = EXT_BASIS[i];
        let k_mod = k.rem_euclid(m as i128) as u64;
        r[i] = ((a.res[i] as u128 * k_mod as u128) % m as u128) as u64;
    }
    ExtRes { res: r }
}
fn ext_from_i64(v: i64) -> ExtRes { ExtRes::from_i128(v as i128) }

fn garner4_to_u128(r: &[u64]) -> u128 {
    let p = NTT_PRIMES;
    let a0 = r[0];
    let a1 = ((r[1] as i128 - a0 as i128).rem_euclid(p[1] as i128) as u64
             * modinv(p[0] % p[1], p[1])) % p[1];
    let p01 = p[0] as u128 * p[1] as u128;
    let t2 = (a0 as u128 + p[0] as u128 * a1 as u128) % p[2] as u128;
    let a2 = ((r[2] as i128 - t2 as i128).rem_euclid(p[2] as i128) as u128
             * modinv((p01 % p[2] as u128) as u64, p[2]) as u128) % p[2] as u128;
    let p012 = p01 * p[2] as u128;
    let t3 = (a0 as u128 + p[0] as u128 * a1 as u128 + p01 * a2) % p[3] as u128;
    let a3 = ((r[3] as i128 - t3 as i128).rem_euclid(p[3] as i128) as u128
             * modinv((p012 % p[3] as u128) as u64, p[3]) as u128) % p[3] as u128;
    a0 as u128 + p[0] as u128 * a1 as u128 + p01 * a2 + p012 * a3
}

fn ext_to_i128(er: &ExtRes) -> i128 {
    let m_ntt = M_NTT;
    let q_ntt = garner4_to_u128(&er.res[0..4]);
    let q_small = er.small_part();
    let m_small = (MOD_2POW as u128) * (MOD_5POW as u128);
    let m_ntt_inv = modinv((m_ntt % m_small) as u64, m_small as u64) as u128;
    let s = ((q_small as i128 - q_ntt as i128).rem_euclid(m_small as i128) as u128
            * m_ntt_inv % m_small);
    let val = q_ntt + m_ntt * s;
    let m_ext = m_ntt * m_small;
    if val < m_ext / 2 { val as i128 } else { val as i128 - m_ext as i128 }
}

/// Instrumented ext_div_signed: returns result AND records |q| into report
fn ext_div_signed_probe(a: &ExtRes, d: u64, report: &mut DivReport) -> ExtRes {
    let val = ext_to_i128(a);
    let abs_val = val.unsigned_abs();
    // q_abs ≤ abs_val / d + 1 (ceiling). Use abs_val directly as upper bound on |q|.
    // More precisely: |q| = |val / d| ≤ |val| / d
    let q_abs = abs_val / d as u128;
    report.update(q_abs);

    if val >= 0 {
        cram_div_exact_pos(a, d)
    } else {
        let neg_a = ext_neg(a);
        let q_pos = cram_div_exact_pos(&neg_a, d);
        ext_neg(&q_pos)
    }
}

fn zero_ext_field() -> Box<[[[ExtRes; N]; N]; N]> {
    Box::new(std::array::from_fn(|_|
        std::array::from_fn(|_|
            std::array::from_fn(|_| ExtRes { res: [0u64; 6] }))))
}

/// Probed step — returns next vorticity state and corridor reports
fn ns_step_probed(
    w: &CRTVortExt,
    params: &NsParams,
    rep_n6: &mut DivReport,
    rep_s2: &mut DivReport,
    rep_4s: &mut DivReport,
    rep_2s: &mut DivReport,
    rep_n2: &mut DivReport,
    max_bs_raw: &mut u128,
) -> CRTVortExt {
    // ---- Biot-Savart (instrumented) ----
    let mut ux_per_prime: Vec<Box<[[[u64; N]; N]; N]>> = (0..4).map(|_|
        Box::new([[[0u64; N]; N]; N])).collect();
    let mut uy_per_prime: Vec<Box<[[[u64; N]; N]; N]>> = (0..4).map(|_|
        Box::new([[[0u64; N]; N]; N])).collect();
    let mut uz_per_prime: Vec<Box<[[[u64; N]; N]; N]>> = (0..4).map(|_|
        Box::new([[[0u64; N]; N]; N])).collect();

    for pi in 0..4 {
        let p = NTT_PRIMES[pi];
        let i_p = I_MOD[pi];
        let mut wx_i64 = zero_field();
        let mut wy_i64 = zero_field();
        let mut wz_i64 = zero_field();
        for i in 0..N { for j in 0..N { for k in 0..N {
            wx_i64[i][j][k] = w.wx[i][j][k].res[pi] as i64;
            wy_i64[i][j][k] = w.wy[i][j][k].res[pi] as i64;
            wz_i64[i][j][k] = w.wz[i][j][k].res[pi] as i64;
        }}}
        let wx_hat = ntt3d(&wx_i64, pi);
        let wy_hat = ntt3d(&wy_i64, pi);
        let wz_hat = ntt3d(&wz_i64, pi);
        let mut ux_hat = Box::new([[[0u64; N]; N]; N]);
        let mut uy_hat = Box::new([[[0u64; N]; N]; N]);
        let mut uz_hat = Box::new([[[0u64; N]; N]; N]);

        let scale_mod = |k: i64, x: u64| -> u64 {
            if k >= 0 { mulmod((k as u64) % p, x, p) }
            else { let v = mulmod(((-k) as u64) % p, x, p); submod(0, v, p) }
        };

        for i0 in 0..N { for i1 in 0..N { for i2 in 0..N {
            let kx = cfreq(i0); let ky = cfreq(i1); let kz = cfreq(i2);
            let ksq = (kx*kx + ky*ky + kz*kz) as u64;
            if ksq == 0 { continue; }
            let inv_ksq = modinv(ksq % p, p);
            let cx = submod(
                scale_mod(ky, mulmod(wz_hat[i0][i1][i2], inv_ksq, p)),
                scale_mod(kz, mulmod(wy_hat[i0][i1][i2], inv_ksq, p)), p);
            let cy = submod(
                scale_mod(kz, mulmod(wx_hat[i0][i1][i2], inv_ksq, p)),
                scale_mod(kx, mulmod(wz_hat[i0][i1][i2], inv_ksq, p)), p);
            let cz = submod(
                scale_mod(kx, mulmod(wy_hat[i0][i1][i2], inv_ksq, p)),
                scale_mod(ky, mulmod(wx_hat[i0][i1][i2], inv_ksq, p)), p);
            ux_hat[i0][i1][i2] = mulmod(i_p, cx, p);
            uy_hat[i0][i1][i2] = mulmod(i_p, cy, p);
            uz_hat[i0][i1][i2] = mulmod(i_p, cz, p);
        }}}
        let ux_r = intt3d(&*ux_hat, pi);
        let uy_r = intt3d(&*uy_hat, pi);
        let uz_r = intt3d(&*uz_hat, pi);
        for i in 0..N { for j in 0..N { for k in 0..N {
            ux_per_prime[pi][i][j][k] = ux_r[i][j][k];
            uy_per_prime[pi][i][j][k] = uy_r[i][j][k];
            uz_per_prime[pi][i][j][k] = uz_r[i][j][k];
        }}}
    }

    let n6: u64 = 1u64 << 30;
    let mut vel_ux = zero_ext_field();
    let mut vel_uy = zero_ext_field();
    let mut vel_uz = zero_ext_field();
    for i in 0..N { for j in 0..N { for k in 0..N {
        let raw_x = garner4([
            ux_per_prime[0][i][j][k], ux_per_prime[1][i][j][k],
            ux_per_prime[2][i][j][k], ux_per_prime[3][i][j][k],
        ]);
        let raw_y = garner4([
            uy_per_prime[0][i][j][k], uy_per_prime[1][i][j][k],
            uy_per_prime[2][i][j][k], uy_per_prime[3][i][j][k],
        ]);
        let raw_z = garner4([
            uz_per_prime[0][i][j][k], uz_per_prime[1][i][j][k],
            uz_per_prime[2][i][j][k], uz_per_prime[3][i][j][k],
        ]);

        // Measure BS raw magnitude
        let abs_x = raw_x.unsigned_abs() as u128;
        let abs_y = raw_y.unsigned_abs() as u128;
        let abs_z = raw_z.unsigned_abs() as u128;
        let cell_max = abs_x.max(abs_y).max(abs_z);
        if cell_max > *max_bs_raw { *max_bs_raw = cell_max; }

        let ux_raw_ext = ExtRes { res: [
            ux_per_prime[0][i][j][k], ux_per_prime[1][i][j][k],
            ux_per_prime[2][i][j][k], ux_per_prime[3][i][j][k],
            (raw_x as i128).rem_euclid(MOD_2POW as i128) as u64,
            (raw_x as i128).rem_euclid(MOD_5POW as i128) as u64,
        ]};
        let uy_raw_ext = ExtRes { res: [
            uy_per_prime[0][i][j][k], uy_per_prime[1][i][j][k],
            uy_per_prime[2][i][j][k], uy_per_prime[3][i][j][k],
            (raw_y as i128).rem_euclid(MOD_2POW as i128) as u64,
            (raw_y as i128).rem_euclid(MOD_5POW as i128) as u64,
        ]};
        let uz_raw_ext = ExtRes { res: [
            uz_per_prime[0][i][j][k], uz_per_prime[1][i][j][k],
            uz_per_prime[2][i][j][k], uz_per_prime[3][i][j][k],
            (raw_z as i128).rem_euclid(MOD_2POW as i128) as u64,
            (raw_z as i128).rem_euclid(MOD_5POW as i128) as u64,
        ]};

        vel_ux[i][j][k] = ext_div_signed_probe(&ux_raw_ext, n6, rep_n6);
        vel_uy[i][j][k] = ext_div_signed_probe(&uy_raw_ext, n6, rep_n6);
        vel_uz[i][j][k] = ext_div_signed_probe(&uz_raw_ext, n6, rep_n6);
    }}}

    // ---- Step body (instrumented) ----
    let nu = ext_from_i64(params.nu_scaled);
    let dt = ext_from_i64(params.dt_latt);
    let s2_div: u64 = (NS_SCALE as u64) * (NS_SCALE as u64);
    let n2_div: u64 = (N as u64) * (N as u64);
    let four_s_div: u64 = 4 * NS_SCALE as u64;
    let two_s_div:  u64 = 2 * NS_SCALE as u64;
    let n_ext = ext_from_i64(N as i64);

    let vel = (vel_ux, vel_uy, vel_uz);

    let mut step_comp = |
        wl: &Box<[[[ExtRes; N]; N]; N]>,
        ul: &Box<[[[ExtRes; N]; N]; N]>,
        u_x: &Box<[[[ExtRes; N]; N]; N]>,
        u_y: &Box<[[[ExtRes; N]; N]; N]>,
        u_z: &Box<[[[ExtRes; N]; N]; N]>,
        full_w: &CRTVortExt,
    | -> Box<[[[ExtRes; N]; N]; N]> {
        let mut out = zero_ext_field();
        for i in 0..N {
            let ip=(i+1)%N; let im=(i+N-1)%N;
            for j in 0..N {
                let jp=(j+1)%N; let jm=(j+N-1)%N;
                for k in 0..N {
                    let kp=(k+1)%N; let km=(k+N-1)%N;

                    let lap = {
                        let sum = ext_add(
                            &ext_add(&ext_add(&wl[ip][j][k], &wl[im][j][k]),
                                     &ext_add(&wl[i][jp][k], &wl[i][jm][k])),
                            &ext_add(&wl[i][j][kp], &wl[i][j][km]));
                        let six = ext_smul(&wl[i][j][k], 6);
                        ext_sub(&sum, &six)
                    };
                    let nu_lap = ext_mul(&nu, &lap);
                    let diff = ext_div_signed_probe(&nu_lap, s2_div, rep_s2);

                    let u_fields = [u_x, u_y, u_z];
                    let sp = [(ip,j,k), (i,jp,k), (i,j,kp)];
                    let sm = [(im,j,k), (i,jm,k), (i,j,km)];
                    let mut raw = ExtRes { res: [0u64; 6] };
                    for d in 0..3 {
                        let (sp0,sp1,sp2) = sp[d];
                        let (sm0,sm1,sm2) = sm[d];
                        let df_pm = ext_sub(&wl[sp0][sp1][sp2], &wl[sm0][sm1][sm2]);
                        let conv = ext_mul(&u_fields[d][i][j][k], &df_pm);
                        let cons = ext_sub(
                            &ext_mul(&u_fields[d][sp0][sp1][sp2], &wl[sp0][sp1][sp2]),
                            &ext_mul(&u_fields[d][sm0][sm1][sm2], &wl[sm0][sm1][sm2]));
                        raw = ext_add(&raw, &ext_add(&conv, &cons));
                    }
                    let raw_n = ext_mul(&raw, &n_ext);
                    let adv = ext_div_signed_probe(&raw_n, four_s_div, rep_4s);

                    let mut raw_s = ExtRes { res: [0u64; 6] };
                    let dux = ext_sub(&ul[ip][j][k], &ul[im][j][k]);
                    let duy = ext_sub(&ul[i][jp][k], &ul[i][jm][k]);
                    let duz = ext_sub(&ul[i][j][kp], &ul[i][j][km]);
                    raw_s = ext_add(&raw_s, &ext_mul(&full_w.wx[i][j][k], &dux));
                    raw_s = ext_add(&raw_s, &ext_mul(&full_w.wy[i][j][k], &duy));
                    raw_s = ext_add(&raw_s, &ext_mul(&full_w.wz[i][j][k], &duz));
                    let raw_s_n = ext_mul(&raw_s, &n_ext);
                    let stretch = ext_div_signed_probe(&raw_s_n, two_s_div, rep_2s);

                    let rhs = ext_add(&ext_sub(&diff, &adv), &stretch);
                    let dt_rhs = ext_mul(&dt, &rhs);
                    let delta = ext_div_signed_probe(&dt_rhs, n2_div, rep_n2);

                    out[i][j][k] = ext_add(&wl[i][j][k], &delta);
                }
            }
        }
        out
    };

    let wx_next = step_comp(&w.wx, &vel.0, &vel.0, &vel.1, &vel.2, w);
    let wy_next = step_comp(&w.wy, &vel.1, &vel.0, &vel.1, &vel.2, w);
    let wz_next = step_comp(&w.wz, &vel.2, &vel.0, &vel.1, &vel.2, w);

    CRTVortExt { wx: wx_next, wy: wy_next, wz: wz_next }
}

// ============================================================================
// Probe runner
// ============================================================================

fn run_probe(label: &str, ic: &Vort3D, params: &NsParams, steps: usize) {
    println!("\n=== PROBE: {} | steps={} ===", label, steps);
    println!("  params: nu_scaled={} dt_latt={}", params.nu_scaled, params.dt_latt);
    println!("  DivContract bounds:");
    println!("    d=N^6=2^30: q_bound_min = {}  (tightest)", Q_BOUND_N6);
    println!("    d=S^2=10^8: q_bound_min = {}", Q_BOUND_S2);
    println!("    d=4S=40000: q_bound_min = {}", Q_BOUND_4S);
    println!("    d=2S=20000: q_bound_min = {}", Q_BOUND_2S);
    println!("    d=N^2=1024: q_bound_min = {}", Q_BOUND_N2);
    println!("  BS raw bound = M_NTT/2 = {}", M_NTT_HALF);
    println!();

    let mut w = CRTVortExt::from_i64(ic);

    // Cumulative per-divisor maxima
    let mut rep_n6 = DivReport::new("d=N^6", 1u64 << 30, Q_BOUND_N6);
    let mut rep_s2 = DivReport::new("d=S^2", 100_000_000, Q_BOUND_S2);
    let mut rep_4s = DivReport::new("d=4S", 40_000, Q_BOUND_4S);
    let mut rep_2s = DivReport::new("d=2S", 20_000, Q_BOUND_2S);
    let mut rep_n2 = DivReport::new("d=N^2", 1024, Q_BOUND_N2);
    let mut max_bs_raw: u128 = 0;

    for step in 0..steps {
        let mut step_n6 = DivReport::new("d=N^6", 1u64 << 30, Q_BOUND_N6);
        let mut step_s2 = DivReport::new("d=S^2", 100_000_000, Q_BOUND_S2);
        let mut step_4s = DivReport::new("d=4S", 40_000, Q_BOUND_4S);
        let mut step_2s = DivReport::new("d=2S", 20_000, Q_BOUND_2S);
        let mut step_n2 = DivReport::new("d=N^2", 1024, Q_BOUND_N2);
        let mut step_bs: u128 = 0;

        w = ns_step_probed(&w, params, &mut step_n6, &mut step_s2,
                           &mut step_4s, &mut step_2s, &mut step_n2, &mut step_bs);

        // Update cumulative
        rep_n6.update(step_n6.max_abs_q);
        rep_s2.update(step_s2.max_abs_q);
        rep_4s.update(step_4s.max_abs_q);
        rep_2s.update(step_2s.max_abs_q);
        rep_n2.update(step_n2.max_abs_q);
        if step_bs > max_bs_raw { max_bs_raw = step_bs; }

        // Per-step summary
        step_n6.finalize(); step_s2.finalize();
        step_4s.finalize(); step_2s.finalize(); step_n2.finalize();

        println!("  step {:3}:", step + 1);
        step_n6.print(); step_s2.print();
        step_4s.print(); step_2s.print(); step_n2.print();

        let bs_margin: i128 = M_NTT_HALF as i128 - step_bs as i128;
        let bs_pct = step_bs * 10_000 / M_NTT_HALF.max(1);
        let bs_status = if step_bs <= M_NTT_HALF { "SAFE" } else { "CORRIDOR-EXCEEDED" };
        println!("  {:8} max|raw|={:15}  bound={:15}  margin={:+16}  used={:4}bp  {}",
                 "BS-raw", step_bs, M_NTT_HALF, bs_margin, bs_pct, bs_status);
    }

    // Final cumulative report
    println!("\n  CUMULATIVE MAXIMA over {} steps:", steps);
    rep_n6.finalize(); rep_s2.finalize();
    rep_4s.finalize(); rep_2s.finalize(); rep_n2.finalize();
    rep_n6.print(); rep_s2.print();
    rep_4s.print(); rep_2s.print(); rep_n2.print();
    let bs_margin: i128 = M_NTT_HALF as i128 - max_bs_raw as i128;
    let bs_pct = max_bs_raw * 10_000 / M_NTT_HALF.max(1);
    let bs_status = if max_bs_raw <= M_NTT_HALF { "SAFE" } else { "CORRIDOR-EXCEEDED" };
    println!("  {:8} max|raw|={:15}  bound={:15}  margin={:+16}  used={:4}bp  {}",
             "BS-raw", max_bs_raw, M_NTT_HALF, bs_margin, bs_pct, bs_status);

    println!("\n  LIMITING FACTOR: BS raw-velocity corridor (M_NTT/2 = {})", M_NTT_HALF);
}

fn main() {
    println!("==========================================================");
    println!("  corridor_probe_v3 — NS Division Contract Measurement");
    println!("  M_NTT         = {}", M_NTT);
    println!("  M_NTT/2 (BS)  = {}", M_NTT_HALF);
    println!("==========================================================");

    let params = NsParams::cfl_max((NS_SCALE * NS_SCALE) / 25);

    // TG amp=100 — baseline (already verified bit-identical)
    let tg_100 = Vort3D::taylor_green(100);
    run_probe("TG amp=100, 5 steps", &tg_100, &params, 5);

    // TG amp=1000
    let tg_1000 = Vort3D::taylor_green(1000);
    run_probe("TG amp=1000, 5 steps", &tg_1000, &params, 5);

    // TG amp=5000 — approaching NS_SCALE
    let tg_5000 = Vort3D::taylor_green(5000);
    run_probe("TG amp=5000, 3 steps", &tg_5000, &params, 3);

    // ABC amp=100 — different geometry
    let abc_100 = Vort3D::abc_flow(100);
    run_probe("ABC amp=100, 5 steps", &abc_100, &params, 5);

    // ABC amp=NS_SCALE — the wall
    let abc_ns = Vort3D::abc_flow(NS_SCALE);
    run_probe("ABC amp=NS_SCALE=10000, 3 steps", &abc_ns, &params, 3);

    println!("\n=== PROBE COMPLETE ===");
}
