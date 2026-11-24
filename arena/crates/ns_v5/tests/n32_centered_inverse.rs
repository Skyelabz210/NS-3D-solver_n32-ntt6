use ns_regularity::ns3d_n32::{Vort3D, N};
use ns_regularity::ns3d_n32_centered_inverse::{centered_integer_velocity, CenteredInverseError};
use ns_regularity::ns3d_n32_ntt6::M_NTT_6;

#[test]
fn centered_inverse_recovers_the_sharp_integer_witness() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_extremizer)
        .unwrap()
        .join()
        .unwrap();
}

fn check_extremizer() {
    for scale in [1, ((M_NTT_6 - 1) / 32) as i64] {
        check_extremizer_at_scale(scale);
    }
}

fn check_extremizer_at_scale(scale: i64) {
    let tent = |j: usize| {
        if j % 2 == 0 {
            (j / 2).min(N / 2 - j / 2) as i64
        } else {
            0
        }
    };
    let diff = |j| tent((j + 1) % N) - tent((j + N - 1) % N);
    let mut w = Vort3D::zero();
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                w.wy[i][j][k] = scale * diff(k);
                w.wz[i][j][k] = -scale * diff(j);
            }
        }
    }
    let u = centered_integer_velocity(&w).unwrap();
    assert_eq!(u.ux[0][0][0], -8 * scale);
    assert_eq!(u.ux[0][N / 2][N / 2], 8 * scale);
    assert!(u.ux.iter().flatten().flatten().all(|&v| v.abs() <= 8 * scale));
    assert!(u.uy.iter().flatten().flatten().all(|&v| v == 0));
    assert!(u.uz.iter().flatten().flatten().all(|&v| v == 0));
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                let center = 4 * (2 - (j % 2 + k % 2) as i64);
                assert_eq!(u.ux[i][j][k], scale * (tent(j) + tent(k) - center));
            }
        }
    }
}

#[test]
fn centered_inverse_checks_integer_parity_and_residue_range() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_rejections)
        .unwrap()
        .join()
        .unwrap();
}

fn check_rejections() {
    let mut w = Vort3D::zero();
    let period_four = [0, 1, 0, -1];
    for plane in &mut w.wx {
        for (j, row) in plane.iter_mut().enumerate() {
            row.fill(period_four[j % 4]);
        }
    }
    let u = centered_integer_velocity(&w).unwrap();
    assert_eq!(&u.uz[0].each_ref().map(|row| row[0])[0..4], &[0, 0, 1, 0]);

    for plane in &mut w.wx {
        for (j, row) in plane.iter_mut().enumerate() {
            row.fill(if j % 2 == 0 { 1 } else { -1 });
        }
    }
    assert!(matches!(centered_integer_velocity(&w), Err(CenteredInverseError::NoIntegerPreimage)));

    w.wx[0][0][0] = (M_NTT_6 / 32 + 1) as i64;
    assert!(matches!(centered_integer_velocity(&w), Err(CenteredInverseError::ResidueRange)));
}

#[test]
fn centered_inverse_rejects_a_fractional_parity_difference() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_fractional_input)
        .unwrap()
        .join()
        .unwrap();
}

fn check_fractional_input() {
    let pulse = |i: usize, j: usize| i64::from(i % 4 == 0 && j % 4 == 0);
    let mut w = Vort3D::zero();
    // Half the curl of curl(0, 0, pulse). Its real velocity has values
    // differing by 1/2 at [0,1,0] and [2,1,0] in the same parity class.
    for i in 0..N {
        for j in 0..N {
            let second_difference = pulse((i + 2) % N, j)
                + pulse((i + N - 2) % N, j)
                + pulse(i, (j + 2) % N)
                + pulse(i, (j + N - 2) % N)
                - 4 * pulse(i, j);
            assert_eq!(second_difference % 2, 0);
            w.wz[i][j].fill(-second_difference / 2);
        }
    }
    assert!(matches!(centered_integer_velocity(&w), Err(CenteredInverseError::NoIntegerPreimage)));
}
