/// Function that returns ```true``` if the input number is a positive to-the-power-of-2 number
/// 
#[inline(always)]
pub fn is_power_of_two(n: u64) -> bool{
    return n > 0 && (n & (n - 1)) == 0;
}

/// Extremely fast 4-point, 3rd-order Hermite interpolation.
/// Optimized for Fused Multiply-Add (FMA) instructions.
#[inline(always)]
pub fn hermite_interp(frac: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let c1 = 0.5 * (p2 - p0);
    let c2 = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c3 = 0.5 * (p3 - p0) + 1.5 * (p1 - p2);
    
    // Horner's method for polynomial evaluation
    ((c3 * frac + c2) * frac + c1) * frac + p1
}