/// Function that returns ```true``` if the input number is a positive to-the-power-of-2 number
/// 
#[inline(always)]
pub fn is_power_of_two(n: u64) -> bool{
    return n > 0 && (n & (n - 1)) == 0;
}