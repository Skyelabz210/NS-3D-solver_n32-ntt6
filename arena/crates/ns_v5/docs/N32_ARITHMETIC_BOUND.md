# N32 production range and rounding bounds

The canonical integer step and CRT v4 step use the same four-prime NTT velocity, then use signed truncation at the diffusion, advection, stretching, and time-update divisions. `production_step_bounds` checks a sufficient range for one step from an `i64` field. It returns a common bound on the absolute output and a rational bound on the pointwise rounding error. The reference evaluates the stencil with the same reconstructed integer velocity and carries all four quotients as rational numbers.

## A pointwise remainder identity

Write $N=32$, $S=10000$, and $D=N^2S^2$. At a site and component let $w$ be the input value, $L$ the seven-point Laplacian without the $N^2$ factor, $A$ the unscaled sum of the advective and conservative advection numerators, and $B$ the unscaled stretching numerator. The exact rational reference is

$$w_* = w + \frac{\Delta t}{N^2}\left(\frac{\nu L}{S^2} - \frac{NA}{4S} + \frac{NB}{2S}\right).$$

Let $r_D=\nu L\bmod_{\mathrm{trunc}} S^2$, $r_A=NA\bmod_{\mathrm{trunc}}4S$, and $r_B=NB\bmod_{\mathrm{trunc}}2S$, where the signed remainders have the Rust `%` convention. Let $Q$ be the signed sum of the three integer quotients and $r_T=\Delta t Q\bmod_{\mathrm{trunc}}N^2$. If the step passes the range checks below, both production outputs equal $w+\operatorname{trunc}(\Delta t Q/N^2)$ and

$$D(w_*-w_{\mathrm{production}})=S^2r_T+\Delta t\left(r_D-\frac S4r_A+\frac S2r_B\right).$$

If the numerator of division by $d$ is a multiple of $a$, the remainder has absolute value at most $d-\gcd(a,d)$. Here $a$ is respectively $|\nu|$, $N$, $N$, and $|\Delta t|$. Substitution gives the implemented pointwise bound purely in integer arithmetic. The certificate charges both stages of truncation, even on a shear where advection and stretching vanish.

The skew advection numerator pairs to zero with vorticity under periodic summation. The conservative shift reindexes into the negative of the advective term. After signed truncation, the pairing equals $-\sum w\,r_A/(4S)$, summed over the grid and components. Its sign can vary with the data. The nonlinear regression exercises this difference by checking zero raw work and nonzero rounded work on the same state.

## Range before applying the bound

Let $W$ be the largest absolute integer vorticity component. The four-prime centered lift is at most $(M-1)/2$ in absolute value, where $M=769\cdot929\cdot1153\cdot1217=1002448197601$. The later $N^6$ division bounds the production velocity components by $U=466$. The canonical Laplacian magnitude is bounded by $12W$. The advection and stretching numerator magnitudes are bounded by $12NUW$ and $6NUW$ respectively. These bounds follow by taking absolute values before summing the six shifts and the three velocity-component products.

The CRT division recovers its nonnegative quotient from the transform lanes and from partial small-lane moduli $m/\gcd(d,m)$. The reconstructable interval for the small-lane lift is $[0,M\,m/\gcd(d,m))$ on each affected lane $m\in\{2^{63},5^{25}\}$. Signed division first factors out the sign. The binding quotient limits for $S^2$, $N^2$, and $N^6$ are $M5^{17}$, $M2^{53}$, and $M2^{33}$ respectively. At the first excluded diffusion quotient $M5^{17}$, the partial fifth-prime lane and the NTT lanes are all zero, so its widened fifth-prime lane differs from the expected one. The test checks this boundary and its immediate predecessor.

The preflight requires $12W\le\mathrm{i64::MAX}$, then checks the diffusion and time lift limits and the bound on the final `i64` value. With $12W\le\mathrm{i64::MAX}$ the divided advection and stretching terms are below `i64::MAX` and their own lift limits for the fixed N32 constants. All signed CRT division inputs admitted by these conditions fit the `i128` reconstruction corridor. The $N^6$ divide also fits, since its input is the centered four-prime representative. The preflight deliberately uses safe upper bounds when cancellations could admit a larger exact step. Rechecking subsequent states certifies each step's local arithmetic error. A global error bound still needs stability and a consistent refining solver family.

## A counterexample at the velocity boundary

Take the shear $\omega_x(y)=(0,1,0,-1)$ repeated on the length-32 axis and $\omega_y=\omega_z=0$. It has only $k_y=\pm8$. Under the implementation's positive-forward NTT convention, the corresponding formal normalized Fourier inverse has velocity $u_z(y)=(1,0,-1,0)/8$ in the code's wave-number units. The same mode can be represented exactly in every transform field since the quarter-period root of unity is the chosen modular square root of $-1$.

At a cosine maximum the four residues encode $1/8$. As $M\equiv 1\pmod8$, the centered integer lift is $(1-M)/8$. The subsequent division by $N^6$ produces $-116$ at that maximum. The inverse transform itself already contains the $N^{-3}$ transform normalization. This example distinguishes recovery of an integer representative from recovery of the rational Fourier value. Canonical/CRT agreement and larger step-body CRT lanes both leave the same velocity discrepancy. A continuum transfer for either production path needs a correctly scaled velocity recovery with a consistency estimate.

## Reproduction

From `arena/`, run both checks with locked dependencies.

```sh
RUST_MIN_STACK=67108864 cargo test --locked --release -p ns_regularity --test n32_roundoff -- --nocapture
RUST_MIN_STACK=67108864 cargo test --locked --release -p ns_regularity --lib ns3d_n32_roundoff
```

The nonlinear fixture uses an ABC field with a deterministic transverse perturbation. It checks both production updates and the independently assembled rational numerator at all 98,304 components. Positive and negative remainders occur at all four division sites. The measured largest error is $338280487069/102400000000$ grid units, below the bound $372383759099/102400000000$. The rounded advection work is $-703667$ in integer grid units. The preflight tests also check conservative rejection for a Laplacian clamp, an out-of-range diffusion lift, an out-of-range time lift, and an out-of-range output.
