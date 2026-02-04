use std::ops::Rem;

pub fn saturating_rem<T: Default + Copy + Eq + Rem<T, Output = T>>(a: T, b: T) -> T {
    let rem = a.rem(b);
    if rem == T::default() { b } else { rem }
}
