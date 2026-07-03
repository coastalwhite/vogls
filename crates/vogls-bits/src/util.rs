use std::ops::Rem;

pub fn saturating_rem<T: Default + Copy + Eq + Rem<T, Output = T>>(a: T, b: T) -> T {
    let rem = a.rem(b);
    if rem == T::default() { b } else { rem }
}

pub fn wrapping_u64_pow(l: u64, r: u64) -> u64 {
    // X**Y
    //      = X**LowerWord(Y) * X**(2**32 * UpperWord(Y))
    //      = X**LowerWord(Y) * (X**UpperWord(Y))**(2**32)
    //      = X**LowerWord(Y) * (X**UpperWord(Y))**(2**16)**2
    let a = l.wrapping_pow((r & 0xFFFF_FFFF) as u32);
    if r < (1 << 32) {
        return a;
    }
    let b = l
        .wrapping_pow((r >> 32) as u32)
        .wrapping_pow(1 << 16)
        .wrapping_pow(1 << 16);
    a.wrapping_mul(b)
}
