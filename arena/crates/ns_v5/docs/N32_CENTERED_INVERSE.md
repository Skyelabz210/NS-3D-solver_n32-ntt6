# An exact integer centered-curl inverse at N32

`centered_integer_velocity` recovers a velocity witness for the raw periodic centered curl and divergence. It solves with the existing six transform primes, sets a separate gauge on each of the eight parity classes, and verifies the two defining equations over the integers. The return type distinguishes an input outside the certified residue range, a singular transform lane, and an input with no integer preimage.

## The operator and its range

Let $D_j u(x)=u(x+e_j)-u(x-e_j)$. In this module the spatial inverse solves $\operatorname{curl}_D u=\omega$ and $\operatorname{div}_D u=0$. For the positive-exponent forward NTT with primitive root $\zeta$, the symbol of $D_j$ is $s_j=\zeta^{-k_j}-\zeta^{k_j}$. At each nonblind frequency the implementation solves

$$\widehat u=\frac{s\times\widehat\omega}{-\sum_j s_j^2}.$$

The code checks the denominator at every nonblind mode in each of the six fields. The eight modes with $k_j\in\{0,16\}$ for all $j$ have $s=0$ and contribute zero to the lane solution. The physical equations reject unsupported components on those modes as well as other vectors outside the integer curl image.

For an admissible integer input, any integer velocity witness solves the same modular system up to constants on the eight parity classes. The implementation therefore subtracts one anchor in each class before decoding. Its centered CRT decode uses parallel idempotent weights $(M/p)\operatorname{inv}_p(M/p)$, where $M$ is the product of the six NTT primes and $p$ is the lane prime. It performs this decode at the checked integer boundary and then centers each component on each parity class by its range midpoint.

The normalized centered-curl inverse has a velocity bound of one half on the unit torus. Since the normalized derivative at N32 is $16D_j$, the unscaled inverse with zero mean on each parity class has $\|u\|_\infty\le8\|\omega\|_\infty$. Thus a parity-anchor difference is at most $16\|\omega\|_\infty$ and is an integer whenever an integer witness exists. Requiring $32\|\omega\|_\infty<M$ puts every such difference in the centered decoding interval. Checking the recovered curl and divergence directly makes success a verifiable statement about the returned field. [Uniform centered Biot–Savart bound](https://github.com/Skyelabz210/NS-Regularity/blob/cbb907723fa5fd42afaeb473a0dd8285724dde19/consolidated/CENTERED_BIOT_SAVART_BOUND.md)

## Verification and integration scope

The exact test cases recover a witness attaining the $8\|\omega\|_\infty$ bound at both unit amplitude and the largest admitted integer amplitude. They also recover a witness for the unit period-four shear and reject a checkerboard input and a real curl with a half-integer difference within a parity class.

```sh
cd arena
cargo test --locked --release -p ns_regularity --test n32_centered_inverse
```

This is an opt-in spatial inverse with a raw centered-difference contract. Using it in a refining evolution requires preserving its input image, retaining the derivative scaling, and independently controlling the nonlinear and rounding errors. The legacy production update uses flat wave-number symbols and truncates its step-body divisions.
