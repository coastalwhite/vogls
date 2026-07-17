use crate::VectorSize;

pub fn tv_leading_zeros(src: &[u64], size: VectorSize) -> u32 {
    let off = size.get() % 64;

    if off == 0 {
        for (i, v) in src.iter().rev().enumerate() {
            if *v != 0 {
                return (i * 64) as u32 + v.leading_zeros();
            }
        }
    } else {
        let last = *src.last().unwrap();
        if last != 0 {
            return last.leading_zeros() - (64 - off);
        }
        for (i, v) in src.iter().rev().skip(1).enumerate() {
            if *v != 0 {
                return (i * 64) as u32 + v.leading_zeros() + off;
            }
        }
    }
    size.get()
}
pub fn tv_leading_ones(src: &[u64], size: VectorSize) -> u32 {
    let off = size.get() % 64;

    if off == 0 {
        for (i, v) in src.iter().rev().enumerate() {
            if *v != !0u64 {
                return (i * 64) as u32 + v.leading_ones();
            }
        }
    } else {
        let last = *src.last().unwrap();
        if last != (1u64 << off).wrapping_sub(1) {
            return (last << (64 - off)).leading_ones();
        }
        for (i, v) in src.iter().rev().skip(1).enumerate() {
            if !*v != 0 {
                return (i * 64) as u32 + v.leading_ones() + off;
            }
        }
    }
    size.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leading_zeros() {
        macro_rules! assert_test_vector {
            ($size:literal, $a:expr, $result:expr) => {
                let result = tv_leading_zeros(&$a, VectorSize::new($size).unwrap());
                assert_eq!(result, $result);
            };
        }

        assert_test_vector!(1, [0u64], 1);
        assert_test_vector!(1, [1u64], 0);
        assert_test_vector!(128, [0u64, 0u64], 128);
        assert_test_vector!(127, [0u64, 0u64], 127);
        assert_test_vector!(128, [!0u64, !0u64], 0);
        assert_test_vector!(128, [!0u64, 0u64], 64);
        assert_test_vector!(128, [!0u64 >> 2, 0u64], 66);
        assert_test_vector!(128, [0u64 >> 2, 4u64], 61);
        assert_test_vector!(101, [0u64 >> 2, 4u64], 61 - 27);
    }

    #[test]
    fn test_leading_ones() {
        macro_rules! assert_test_vector {
            ($size:literal, $a:expr, $result:expr) => {
                let result = tv_leading_ones(&$a, VectorSize::new($size).unwrap());
                assert_eq!(result, $result);
            };
        }

        assert_test_vector!(1, [0u64], 0);
        assert_test_vector!(1, [1u64], 1);
        assert_test_vector!(128, [!0u64, !0u64], 128);
        assert_test_vector!(127, [!0u64, !0u64 >> 1], 127);
        assert_test_vector!(128, [0u64, 0u64], 0);
        assert_test_vector!(128, [0u64, !0u64], 64);
        assert_test_vector!(128, [!0u64 << 62, !0u64], 66);
        assert_test_vector!(128, [!0u64 >> 2, !4u64], 61);
        assert_test_vector!(101, [!0u64 >> 2, !4u64 & (!0 >> 27)], 61 - 27);
    }
}
