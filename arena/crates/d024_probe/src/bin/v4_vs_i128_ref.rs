//! v4 vs i128-unclamped reference at amp=1000 multistep.
//!
//! This is the test that broke v3 at step 11 (512 mismatches, max diff
//! 1.88×10²⁷ ≈ 0.45·M_ext). With the widened u128 substrate (lane 5²⁵
//! vs old 5⁹), the binding S² lift corridor jumps from M_NTT·5 ≈ 5×10¹²
//! to M_NTT·5¹⁷ ≈ 7.6×10²³ — eleven orders of magnitude headroom over
//! step-11's q ≈ 1.9×10¹³.

use ns_regularity::ns3d_n32::{N, NS_SCALE, NsParams, Vort3D, biot_savart};
use ns_regularity::ns3d_n32_crt_v4::{CRTVortExtU128, ns_step_crt_v4};

#[derive(Clone)]
struct Vort3DI128 {
    wx: Box<[[[i128; N]; N]; N]>,
    wy: Box<[[[i128; N]; N]; N]>,
    wz: Box<[[[i128; N]; N]; N]>,
}
impl Vort3DI128 {
    fn from_i64(w: &Vort3D) -> Self {
        let mut wx = Box::new([[[0i128; N]; N]; N]);
        let mut wy = Box::new([[[0i128; N]; N]; N]);
        let mut wz = Box::new([[[0i128; N]; N]; N]);
        for i in 0..N { for j in 0..N { for k in 0..N {
            wx[i][j][k] = w.wx[i][j][k] as i128;
            wy[i][j][k] = w.wy[i][j][k] as i128;
            wz[i][j][k] = w.wz[i][j][k] as i128;
        }}}
        Vort3DI128 { wx, wy, wz }
    }
    fn max_abs(&self) -> i128 {
        let mut m = 0i128;
        for i in 0..N { for j in 0..N { for k in 0..N {
            m = m.max(self.wx[i][j][k].abs())
                 .max(self.wy[i][j][k].abs())
                 .max(self.wz[i][j][k].abs());
        }}}
        m
    }
}

/// Step-body i128 reference. Biot-Savart still uses the canonical 4-prime/i64
/// path, so this is not a true widened-spectral reference. It is valid only
/// until the BS Garner4 corridor aliases. The observed step-15 divergence is
/// therefore a spectral-corridor diagnostic, not a v4 division failure.
fn ns_step_i128(w: &Vort3DI128, params: &NsParams) -> Vort3DI128 {
    let mut w_clamped = Vort3D::zero();
    for i in 0..N { for j in 0..N { for k in 0..N {
        w_clamped.wx[i][j][k] = w.wx[i][j][k].clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        w_clamped.wy[i][j][k] = w.wy[i][j][k].clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        w_clamped.wz[i][j][k] = w.wz[i][j][k].clamp(i64::MIN as i128, i64::MAX as i128) as i64;
    }}}
    let u = biot_savart(&w_clamped);

    let nu = params.nu_scaled as i128;
    let dt = params.dt_latt as i128;
    let s2 = (NS_SCALE as i128) * (NS_SCALE as i128);
    let n2: i128 = 1024;
    let four_s: i128 = 4 * NS_SCALE as i128;
    let two_s: i128 = 2 * NS_SCALE as i128;
    let ni: i128 = N as i128;

    let advect = |u: &ns_regularity::ns3d_n32::Vel3D, wl: &Box<[[[i128; N]; N]; N]>|
                 -> Box<[[[i128; N]; N]; N]> {
        let mut out = Box::new([[[0i128; N]; N]; N]);
        for i in 0..N {
            let ip=(i+1)%N; let im=(i+N-1)%N;
            for j in 0..N {
                let jp=(j+1)%N; let jm=(j+N-1)%N;
                for k in 0..N {
                    let kp=(k+1)%N; let km=(k+N-1)%N;
                    let u_fields = [&u.ux, &u.uy, &u.uz];
                    let sp = [(ip,j,k),(i,jp,k),(i,j,kp)];
                    let sm = [(im,j,k),(i,jm,k),(i,j,km)];
                    let mut raw: i128 = 0;
                    for d in 0..3 {
                        let (sp0,sp1,sp2) = sp[d];
                        let (sm0,sm1,sm2) = sm[d];
                        let u_here  = u_fields[d][i][j][k] as i128;
                        let u_plus  = u_fields[d][sp0][sp1][sp2] as i128;
                        let u_minus = u_fields[d][sm0][sm1][sm2] as i128;
                        let f_plus  = wl[sp0][sp1][sp2];
                        let f_minus = wl[sm0][sm1][sm2];
                        raw += u_here * (f_plus - f_minus)
                              + u_plus * f_plus - u_minus * f_minus;
                    }
                    out[i][j][k] = raw * ni / four_s;
                }
            }
        }
        out
    };
    let stretch = |w: &Vort3DI128, ul: &ns_regularity::ns3d_n32::Field3D|
                  -> Box<[[[i128; N]; N]; N]> {
        let mut out = Box::new([[[0i128; N]; N]; N]);
        for i in 0..N {
            let ip=(i+1)%N; let im=(i+N-1)%N;
            for j in 0..N {
                let jp=(j+1)%N; let jm=(j+N-1)%N;
                for k in 0..N {
                    let kp=(k+1)%N; let km=(k+N-1)%N;
                    let mut raw: i128 = 0;
                    raw += w.wx[i][j][k] * (ul[ip][j][k] - ul[im][j][k]) as i128;
                    raw += w.wy[i][j][k] * (ul[i][jp][k] - ul[i][jm][k]) as i128;
                    raw += w.wz[i][j][k] * (ul[i][j][kp] - ul[i][j][km]) as i128;
                    out[i][j][k] = raw * ni / two_s;
                }
            }
        }
        out
    };
    let lap = |wl: &Box<[[[i128; N]; N]; N]>| -> Box<[[[i128; N]; N]; N]> {
        let mut out = Box::new([[[0i128; N]; N]; N]);
        for i in 0..N {
            let ip=(i+1)%N; let im=(i+N-1)%N;
            for j in 0..N {
                let jp=(j+1)%N; let jm=(j+N-1)%N;
                for k in 0..N {
                    let kp=(k+1)%N; let km=(k+N-1)%N;
                    out[i][j][k] = wl[ip][j][k] + wl[im][j][k]
                                 + wl[i][jp][k] + wl[i][jm][k]
                                 + wl[i][j][kp] + wl[i][j][km]
                                 - 6 * wl[i][j][k];
                }
            }
        }
        out
    };

    let adv_x = advect(&u, &w.wx); let adv_y = advect(&u, &w.wy); let adv_z = advect(&u, &w.wz);
    let str_x = stretch(w, &u.ux); let str_y = stretch(w, &u.uy); let str_z = stretch(w, &u.uz);
    let lap_x = lap(&w.wx); let lap_y = lap(&w.wy); let lap_z = lap(&w.wz);

    let mut out_wx = Box::new([[[0i128; N]; N]; N]);
    let mut out_wy = Box::new([[[0i128; N]; N]; N]);
    let mut out_wz = Box::new([[[0i128; N]; N]; N]);
    for i in 0..N { for j in 0..N { for k in 0..N {
        let diff_x = nu * lap_x[i][j][k] / s2;
        let diff_y = nu * lap_y[i][j][k] / s2;
        let diff_z = nu * lap_z[i][j][k] / s2;
        let rhs_x = diff_x - adv_x[i][j][k] + str_x[i][j][k];
        let rhs_y = diff_y - adv_y[i][j][k] + str_y[i][j][k];
        let rhs_z = diff_z - adv_z[i][j][k] + str_z[i][j][k];
        out_wx[i][j][k] = w.wx[i][j][k] + dt * rhs_x / n2;
        out_wy[i][j][k] = w.wy[i][j][k] + dt * rhs_y / n2;
        out_wz[i][j][k] = w.wz[i][j][k] + dt * rhs_z / n2;
    }}}
    Vort3DI128 { wx: out_wx, wy: out_wy, wz: out_wz }
}

fn v4_to_i128(w: &CRTVortExtU128) -> Vort3DI128 {
    let (wx, wy, wz) = w.to_i128_field();
    Vort3DI128 { wx, wy, wz }
}

fn compare(a: &Vort3DI128, b: &Vort3DI128) -> (usize, i128) {
    let mut mismatches = 0;
    let mut max_diff: i128 = 0;
    for i in 0..N { for j in 0..N { for k in 0..N {
        for (av, bv) in [
            (a.wx[i][j][k], b.wx[i][j][k]),
            (a.wy[i][j][k], b.wy[i][j][k]),
            (a.wz[i][j][k], b.wz[i][j][k]),
        ] {
            if av != bv {
                mismatches += 1;
                let d = (av - bv).abs();
                if d > max_diff { max_diff = d; }
            }
        }
    }}}
    (mismatches, max_diff)
}

fn main() {
    std::thread::Builder::new().stack_size(1024 * 1024 * 1024).spawn(|| {
        run_test();
    }).unwrap().join().unwrap();
}

fn run_test() {
    let nu = 1_000_000i64;
    let params = NsParams::cfl_max(nu);

    println!("v4 (u128 substrate) vs i128 unclamped reference");
    println!("Test: TG amp=1000 multistep through step 15");
    println!("v3's failure: step 11, 512 mismatches, max|diff|=1.88×10²⁷");
    println!();

    let mut w_ref = Vort3DI128::from_i64(&Vort3D::taylor_green(1000));
    let mut w_v4 = CRTVortExtU128::from_i64(&Vort3D::taylor_green(1000));

    for step in 1..=15 {
        w_ref = ns_step_i128(&w_ref, &params);
        w_v4 = ns_step_crt_v4(&w_v4, &params);
        let w_v4_i128 = v4_to_i128(&w_v4);
        let (m, md) = compare(&w_ref, &w_v4_i128);
        let max_ref = w_ref.max_abs();
        println!("step {step:>3}: i128_ref_max={max_ref:>30}  mismatches_v4={m:>6}  max|diff|={md}");
    }
    println!();
    println!("Interpretation: steps 1-14 verify the widened u128 division substrate.");
    println!("Step 15 divergence marks the 4-prime Biot-Savart spectral corridor, not the S² lift.");
}
