use ns_regularity::ns3d_n32::{
    biot_savart, energy_residual_exact, ns_step, NsParams, Vort3D, N, NS_SCALE, NTT_MODULUS,
};
use ns_regularity::ns3d_n32_crt_v4::{ns_step_crt_v4, CRTVortExtU128};
use ns_regularity::ns3d_n32_roundoff::{
    production_step_bounds, N32StepBoundError, MAX_RECONSTRUCTED_VELOCITY,
};

#[test]
fn nonlinear_production_roundoff_against_exact_rational_stencil() {
    // The legacy solvers use full-grid arrays on the stack.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_nonlinear_stencil)
        .unwrap()
        .join()
        .unwrap();
}

fn check_nonlinear_stencil() {
    let mut w = Vort3D::abc_flow(10_007);
    for plane in &mut w.wx {
        for (j, row) in plane.iter_mut().enumerate() {
            for (k, value) in row.iter_mut().enumerate() {
                *value += ((37 * j * k + 11 * j + 5 * k) % 701) as i64 - 350;
            }
        }
    }
    let params = NsParams {
        nu_scaled: 16_666_669,
        dt_latt: 901,
    };
    let bounds = production_step_bounds(&w, &params).unwrap();
    let u = biot_savart(&w);
    let canonical = ns_step(&w, &params);
    let crt = ns_step_crt_v4(&CRTVortExtU128::from_i64(&w), &params).to_i128_field();
    let velocities = [&u.ux, &u.uy, &u.uz];
    let vorticities = [&w.wx, &w.wy, &w.wz];
    let canonical = [&canonical.wx, &canonical.wy, &canonical.wz];
    let crt = [&crt.0, &crt.1, &crt.2];
    let n = N as i128;
    let scale = i128::from(NS_SCALE);
    let den = n.pow(2) * scale.pow(2);
    let adv_factor = scale / 4;
    let stretch_factor = scale / 2;
    let dt = i128::from(params.dt_latt);
    let nu = i128::from(params.nu_scaled);
    let mut signed_remainders = [(false, false); 4];
    let mut max_error = 0;
    let mut unrounded_advection_work = 0;
    let mut advection_remainder_work = 0;

    for c in 0..3 {
        for i in 0..N {
            for j in 0..N {
                for k in 0..N {
                    let at = [i, j, k];
                    let read = |field: &[[[i64; N]; N]; N], [x, y, z]: [usize; 3]| {
                        i128::from(field[x][y][z])
                    };
                    let wc = read(vorticities[c], at);
                    let mut lap = -6 * wc;
                    let mut adv = 0;
                    let mut stretch = 0;
                    for d in 0..3 {
                        let mut plus = at;
                        let mut minus = at;
                        plus[d] = (at[d] + 1) % N;
                        minus[d] = (at[d] + N - 1) % N;
                        let fp = read(vorticities[c], plus);
                        let fm = read(vorticities[c], minus);
                        let up = read(velocities[d], plus);
                        let um = read(velocities[d], minus);
                        lap += fp + fm;
                        adv += read(velocities[d], at) * (fp - fm) + up * fp - um * fm;
                        stretch += read(vorticities[d], at)
                            * (read(velocities[c], plus) - read(velocities[c], minus));
                    }
                    let diff_num = nu * lap;
                    let adv_num = n * adv;
                    let stretch_num = n * stretch;
                    let exact = wc * den
                        + dt * (diff_num - adv_factor * adv_num + stretch_factor * stretch_num);
                    let inner = diff_num / scale.pow(2) - adv_num / (4 * scale)
                        + stretch_num / (2 * scale);
                    let remainders = [
                        diff_num % scale.pow(2),
                        adv_num % (4 * scale),
                        stretch_num % (2 * scale),
                        (dt * inner) % n.pow(2),
                    ];
                    unrounded_advection_work += wc * adv_num;
                    advection_remainder_work += wc * remainders[1];
                    for ((positive, negative), remainder) in
                        signed_remainders.iter_mut().zip(remainders)
                    {
                        *positive |= remainder > 0;
                        *negative |= remainder < 0;
                    }
                    let expected_error = scale.pow(2) * remainders[3]
                        + dt * (remainders[0] - adv_factor * remainders[1]
                            + stretch_factor * remainders[2]);
                    let actual = i128::from(canonical[c][i][j][k]);
                    let error = exact - den * actual;
                    assert_eq!(actual, crt[c][i][j][k]);
                    assert_eq!(error, expected_error);
                    assert!(actual.unsigned_abs() <= bounds.max_output);
                    max_error = max_error.max(error.unsigned_abs());
                }
            }
        }
    }

    assert_eq!(bounds.error_denominator, den as u128);
    assert_eq!(signed_remainders, [(true, true); 4]);
    assert!(max_error > 0 && max_error <= bounds.error_numerator);
    assert_eq!(unrounded_advection_work, 0);
    assert_ne!(advection_remainder_work, 0);
    assert_eq!(
        4 * scale * energy_residual_exact(&u, &w),
        -advection_remainder_work
    );
    eprintln!("rounded advection remainder work {advection_remainder_work}");
    eprintln!(
        "largest error {max_error}/{den}, bound {}/{den}",
        bounds.error_numerator
    );
}

#[test]
fn preflight_checks_the_conservative_integer_range() {
    let mut w = Vort3D::zero();
    let zero_step = NsParams {
        nu_scaled: i64::MIN,
        dt_latt: 0,
    };
    assert_eq!(
        production_step_bounds(&w, &zero_step).unwrap().error_numerator,
        0
    );

    w.wx[0][0][0] = i64::MIN;
    assert_eq!(
        production_step_bounds(&w, &zero_step),
        Err(N32StepBoundError::LaplacianRange)
    );

    w.wx[0][0][0] = i64::MAX / 12;
    assert_eq!(
        production_step_bounds(&w, &zero_step),
        Err(N32StepBoundError::DiffusionLiftRange)
    );

    let large_step = NsParams {
        nu_scaled: 0,
        dt_latt: i64::MAX,
    };
    assert_eq!(
        production_step_bounds(&w, &large_step),
        Err(N32StepBoundError::TimeLiftRange)
    );

    let large_update = NsParams {
        nu_scaled: 0,
        dt_latt: 10_000,
    };
    assert_eq!(
        production_step_bounds(&w, &large_update),
        Err(N32StepBoundError::OutputRange)
    );
}

#[test]
fn normalized_ntt_lift_does_not_decode_rational_velocity() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_fourier_mode)
        .unwrap()
        .join()
        .unwrap();
}

fn check_fourier_mode() {
    let mut w = Vort3D::zero();
    let period_four_sine = [0, 1, 0, -1];
    for plane in &mut w.wx {
        for (j, row) in plane.iter_mut().enumerate() {
            row.fill(period_four_sine[j % 4]);
        }
    }
    let u = biot_savart(&w);
    let n6 = (N as i64).pow(6);
    let modulus = NTT_MODULUS as i64;
    assert_eq!(modulus % 8, 1);
    let lifted_eighth = (1 - modulus) / 8;
    assert_eq!((8 * lifted_eighth).rem_euclid(modulus), 1);
    assert_eq!(MAX_RECONSTRUCTED_VELOCITY, 466);
    assert_eq!(lifted_eighth / n6, -116);
    let expected = [lifted_eighth / n6, 0, -lifted_eighth / n6, 0];
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                assert_eq!((u.ux[i][j][k], u.uy[i][j][k]), (0, 0));
                assert_eq!(u.uz[i][j][k], expected[j % 4]);
            }
        }
    }
}
