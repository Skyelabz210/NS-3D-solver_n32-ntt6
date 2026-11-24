//! Checked integer inverse of the raw N32 centered curl on its integer image.

use crate::ns3d::{addmod, modinv, modpow, mulmod, submod};
use crate::ns3d_n32::{Field3D, Vel3D, Vort3D, N};
use crate::ns3d_n32_ntt6::{intt3d_ext, ntt3d_ext, M_NTT_6, NTT_PRIMES_6, NTT_ROOTS_6};

type ModField = [[[u64; N]; N]; N];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CenteredInverseError {
    ResidueRange,
    SingularTransformLane,
    NoIntegerPreimage,
}

/// Recover an integer velocity witness for the raw centered curl.
/// On success the returned velocity has exact centered divergence zero and
/// exactly the supplied curl. Parity constants are chosen by range centering.
pub fn centered_integer_velocity(w: &Vort3D) -> Result<Vel3D, CenteredInverseError> {
    let fields = [&w.wx, &w.wy, &w.wz];
    let max_w = fields
        .iter()
        .flat_map(|field| field.iter().flatten().flatten())
        .map(|value| u128::from(value.unsigned_abs()))
        .max()
        .unwrap_or(0);
    // For representable data the parity-anchored raw inverse is bounded
    // by 16*max_w. Widely separated aliases would defeat completeness.
    if 32 * max_w >= M_NTT_6 {
        return Err(CenteredInverseError::ResidueRange);
    }

    let mut lanes = Vec::with_capacity(NTT_PRIMES_6.len());
    for lane in 0..NTT_PRIMES_6.len() {
        lanes.push(invert_lane(fields, lane)?);
    }
    let weights = NTT_PRIMES_6.map(|p| {
        let m = M_NTT_6 / u128::from(p);
        m * u128::from(modinv((m % u128::from(p)) as u64, p))
    });
    let recovered: [Field3D; 3] = std::array::from_fn(|component| {
        let mut relative = [[[0; N]; N]; N];
        let mut lo = [0i64; 8];
        let mut hi = [0i64; 8];
        for (i, plane) in relative.iter_mut().enumerate() {
            for (j, row) in plane.iter_mut().enumerate() {
                for (k, value) in row.iter_mut().enumerate() {
                    let anchor = [i % 2, j % 2, k % 2];
                    let combined: u128 = lanes
                        .iter()
                        .zip(weights)
                        .enumerate()
                        .map(|(lane, (fields, weight))| {
                            let grid = &fields[component];
                            let difference = submod(
                                grid[i][j][k],
                                grid[anchor[0]][anchor[1]][anchor[2]],
                                NTT_PRIMES_6[lane],
                            );
                            u128::from(difference) * weight
                        })
                        .sum::<u128>() % M_NTT_6;
                    *value = if combined <= M_NTT_6 / 2 {
                        combined as i64
                    } else {
                        -((M_NTT_6 - combined) as i64)
                    };
                    let parity = parity_index(i, j, k);
                    lo[parity] = lo[parity].min(*value);
                    hi[parity] = hi[parity].max(*value);
                }
            }
        }
        for (i, plane) in relative.iter_mut().enumerate() {
            for (j, row) in plane.iter_mut().enumerate() {
                for (k, value) in row.iter_mut().enumerate() {
                    let parity = parity_index(i, j, k);
                    *value -= (lo[parity] + hi[parity]).div_euclid(2);
                }
            }
        }
        relative
    });
    verify_witness(&recovered, fields)?;
    let [ux, uy, uz] = recovered;
    Ok(Vel3D { ux, uy, uz })
}

fn invert_lane(fields: [&Field3D; 3], lane: usize) -> Result<[Box<ModField>; 3], CenteredInverseError> {
    let p = NTT_PRIMES_6[lane];
    let root = NTT_ROOTS_6[lane];
    let symbol: [u64; N] = std::array::from_fn(|k| {
        // The existing forward transform has positive exponents.
        submod(modpow(root, ((N - k) % N) as u64, p), modpow(root, k as u64, p), p)
    });
    let w_hat = fields.map(|field| ntt3d_ext(field, lane));
    let mut u_hat: [Box<ModField>; 3] = std::array::from_fn(|_| Box::new([[[0; N]; N]; N]));
    for (i, &sx) in symbol.iter().enumerate() {
        for (j, &sy) in symbol.iter().enumerate() {
            for (k, &sz) in symbol.iter().enumerate() {
                let s = [sx, sy, sz];
                if s == [0; 3] {
                    continue;
                }
                let squared = s.iter().map(|&d| mulmod(d, d, p)).fold(0, |acc, d| addmod(acc, d, p));
                if squared == 0 {
                    return Err(CenteredInverseError::SingularTransformLane);
                }
                let inverse_lap = modinv(p - squared, p);
                for (c, field) in u_hat.iter_mut().enumerate() {
                    let d = (c + 1) % 3;
                    let e = (c + 2) % 3;
                    let cross = submod(mulmod(s[d], w_hat[e][i][j][k], p), mulmod(s[e], w_hat[d][i][j][k], p), p);
                    field[i][j][k] = mulmod(cross, inverse_lap, p);
                }
            }
        }
    }
    Ok(u_hat.map(|field| intt3d_ext(&field, lane)))
}

fn parity_index(i: usize, j: usize, k: usize) -> usize {
    ((i % 2) << 2) | ((j % 2) << 1) | (k % 2)
}

fn verify_witness(u: &[Field3D; 3], w: [&Field3D; 3]) -> Result<(), CenteredInverseError> {
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                let at = [i, j, k];
                let mut du = [[0i128; 3]; 3];
                for d in 0..3 {
                    let mut plus = at;
                    let mut minus = at;
                    plus[d] = (at[d] + 1) % N;
                    minus[d] = (at[d] + N - 1) % N;
                    for c in 0..3 {
                        du[d][c] = i128::from(u[c][plus[0]][plus[1]][plus[2]])
                            - i128::from(u[c][minus[0]][minus[1]][minus[2]]);
                    }
                }
                let curl = [du[1][2] - du[2][1], du[2][0] - du[0][2], du[0][1] - du[1][0]];
                if du[0][0] + du[1][1] + du[2][2] != 0
                    || curl != w.map(|component| i128::from(component[i][j][k]))
                {
                    return Err(CenteredInverseError::NoIntegerPreimage);
                }
            }
        }
    }
    Ok(())
}
