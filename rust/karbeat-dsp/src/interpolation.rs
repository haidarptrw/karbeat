use num_traits::Float;

/// Extremely fast 4-point, 3rd-order Hermite interpolation.
/// Optimized for Fused Multiply-Add (FMA) instructions.
#[inline(always)]
pub fn hermite_interp_f32(frac: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let c1 = 0.5 * (p2 - p0);
    let c2 = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c3 = 0.5 * (p3 - p0) + 1.5 * (p1 - p2);

    // Horner’s method with fused multiply-add
    c3.mul_add(frac, c2).mul_add(frac, c1).mul_add(frac, p1)
}

#[inline(always)]
pub fn hermite_intern_f64(frac: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let c1 = 0.5 * (p2 - p0);
    let c2 = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c3 = 0.5 * (p3 - p0) + 1.5 * (p1 - p2);

    // Horner's method for polynomial evaluation
    ((c3 * frac + c2) * frac + c1) * frac + p1
}

#[inline(always)]
pub fn lerp<T: Float>(alpha: T, p0: T, p1: T) -> T {
    (p1 - p0).mul_add(alpha, p0)
}
