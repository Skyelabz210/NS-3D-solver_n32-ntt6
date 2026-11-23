//! One-step arithmetic bounds for the canonical and CRT v4 N32 solvers.
//!
//! These bounds compare with the rational evaluation of the active stencil
//! with the common integer velocity from the existing NTT reconstruction.

use crate::cram_div_u128::{LANE_2POW, LANE_5POW, M_NTT_U128};
use crate::ns3d_n32::{NsParams, Vort3D, N, NTT_MODULUS, NS_SCALE};

const N2: u128 = (N * N) as u128;
const N6: u128 = N2 * N2 * N2;
const S2: u128 = (NS_SCALE as u128).pow(2);
const ADVECTION_DIVISOR: u128 = 4 * NS_SCALE as u128;
const STRETCHING_DIVISOR: u128 = 2 * NS_SCALE as u128;
const I64_LIMIT: u128 = i64::MAX as u128;

/// Absolute component bound after centered four-prime lift and N^6 division.
pub const MAX_RECONSTRUCTED_VELOCITY: u128 = (NTT_MODULUS as u128 / 2) / N6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct N32StepBounds {
    /// Upper bound for the absolute output components of either solver.
    pub max_output: u128,
    /// Numerator of the pointwise absolute rounding-error bound.
    pub error_numerator: u128,
    /// Common denominator for the pointwise absolute rounding-error bound.
    pub error_denominator: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum N32StepBoundError {
    LaplacianRange,
    DiffusionLiftRange,
    TimeLiftRange,
    OutputRange,
}

/// Certify a single step from an i64 state. A failed bound means this
/// sufficient test cannot certify the input. Reapply to the actual next
/// state to certify a multistep trajectory.
pub fn production_step_bounds(
    w: &Vort3D,
    params: &NsParams,
) -> Result<N32StepBounds, N32StepBoundError> {
    let max_w = [&w.wx, &w.wy, &w.wz]
        .into_iter()
        .flat_map(|component| component.iter().flatten().flatten())
        .map(|value| u128::from(value.unsigned_abs()))
        .max()
        .unwrap_or(0);
    let nu = u128::from(params.nu_scaled.unsigned_abs());
    let dt = u128::from(params.dt_latt.unsigned_abs());

    let laplacian = 12 * max_w;
    if laplacian > I64_LIMIT {
        return Err(N32StepBoundError::LaplacianRange);
    }
    let diffusion = nu * laplacian / S2;
    if diffusion >= quotient_lift_limit(S2) {
        return Err(N32StepBoundError::DiffusionLiftRange);
    }

    let mixed = N as u128 * MAX_RECONSTRUCTED_VELOCITY * max_w;
    let advection = 12 * mixed / ADVECTION_DIVISOR;
    let stretching = 6 * mixed / STRETCHING_DIVISOR;
    // With the N32 constants and 12*max_w <= i64::MAX, both divided
    // nonlinear terms fit i64 and are inside their CRT lift corridors.
    let time_numerator = dt
        .checked_mul(diffusion + advection + stretching)
        .ok_or(N32StepBoundError::TimeLiftRange)?;
    let time_increment = time_numerator / N2;
    if time_increment >= quotient_lift_limit(N2) {
        return Err(N32StepBoundError::TimeLiftRange);
    }
    let max_output = max_w + time_increment;
    if max_output > I64_LIMIT {
        return Err(N32StepBoundError::OutputRange);
    }

    let diffusion_rem = S2 - gcd(nu, S2);
    let advection_rem = ADVECTION_DIVISOR - gcd(N as u128, ADVECTION_DIVISOR);
    let stretching_rem = STRETCHING_DIVISOR - gcd(N as u128, STRETCHING_DIVISOR);
    let time_rem = N2 - gcd(dt, N2);
    let inner = diffusion_rem
        + (S2 / ADVECTION_DIVISOR) * advection_rem
        + (S2 / STRETCHING_DIVISOR) * stretching_rem;

    Ok(N32StepBounds {
        max_output,
        error_numerator: dt * inner + S2 * time_rem,
        error_denominator: N2 * S2,
    })
}

fn quotient_lift_limit(divisor: u128) -> u128 {
    [LANE_2POW, LANE_5POW]
        .into_iter()
        .filter_map(|lane| {
            let common = gcd(divisor, lane);
            (common > 1).then_some(M_NTT_U128 * (lane / common))
        })
        .min()
        .unwrap_or(u128::MAX)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cram_div_u128::{cram_div_exact_pos_u128, ExtResU128};

    #[test]
    fn diffusion_quotient_lift_is_ambiguous_at_the_first_alias() {
        let limit = quotient_lift_limit(S2);
        let encode = |q| ExtResU128::from_i128((q * S2) as i128);
        let just_inside = cram_div_exact_pos_u128(&encode(limit - 1), S2);
        assert_eq!(just_inside, ExtResU128::from_i128((limit - 1) as i128));

        let alias = cram_div_exact_pos_u128(&encode(limit), S2);
        assert_eq!(alias.res[5], 0);
        assert_ne!(alias, ExtResU128::from_i128(limit as i128));
    }
}
