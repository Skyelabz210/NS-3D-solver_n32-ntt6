//! A centered curl on a periodic shear with sub-unit diffusion corrections.
use ns_regularity::ns3d_n32::{ns_step, NsParams, Vort3D, N, NS_SCALE};
use ns_regularity::ns3d_n32_crt_v4::{ns_step_crt_v4, CRTVortExtU128};

#[test]
fn shear_with_negative_fractional_diffusion() {
    // Solver entry points allocate several fixed-size N32 arrays on the stack.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_shear)
        .unwrap()
        .join()
        .unwrap();
}

fn check_shear() {
    let mut input = Vort3D::zero();
    for plane in &mut input.wx {
        // curl_x of u_z = 1 at y=0 and zero elsewhere, with centered differences.
        plane[N - 1].fill(1);
        plane[1].fill(-1);
    }

    let params = NsParams { nu_scaled: 1, dt_latt: 1 };
    let canonical = ns_step(&input, &params);
    let encoded = CRTVortExtU128::from_i64(&input);
    let crt = ns_step_crt_v4(&encoded, &params).to_i128_field();
    let denominator = (N as i64 * NS_SCALE).pow(2);
    let mut floor_mass = 0;
    let mut exact_parity_sums = [0_i128; 8];
    let mut max_rust_error_numerator = 0_i128;
    let mut max_floor_error_numerator = 0_i128;
    let scale = i128::from(denominator);
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                let center = input.wx[i][j][k];
                let lap = input.wx[i][(j + 1) % N][k] + input.wx[i][(j + N - 1) % N][k]
                    - 2 * center;
                let floor = center + lap.div_euclid(denominator);
                let exact = scale * i128::from(center) + i128::from(lap);
                let parity = ((i % 2) << 2) | ((j % 2) << 1) | (k % 2);
                exact_parity_sums[parity] += exact;
                floor_mass += floor;
                let rust_error = (scale * i128::from(canonical.wx[i][j][k]) - exact).abs();
                let floor_error = (scale * i128::from(floor) - exact).abs();
                max_rust_error_numerator = max_rust_error_numerator.max(rust_error);
                max_floor_error_numerator = max_floor_error_numerator.max(floor_error);
                assert_eq!(canonical.wx[i][j][k], center);
                assert_eq!(crt.0[i][j][k], i128::from(center));
                assert_eq!((canonical.wy[i][j][k], canonical.wz[i][j][k]), (0, 0));
                assert_eq!((crt.1[i][j][k], crt.2[i][j][k]), (0, 0));
            }
        }
    }
    assert_eq!(floor_mass, -2 * (N as i64).pow(2));
    assert_eq!(exact_parity_sums, [0; 8]);
    assert_eq!(max_rust_error_numerator, 2);
    assert_eq!(max_floor_error_numerator, scale - 1);
}
