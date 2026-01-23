use std::cmp::Ordering;

use crate::VectorSize;
use crate::comparison::tv_gtu64_unsigned_leq;
use crate::leading_trailing::tv_leading_zeros;
use crate::load::load_partial_u64;
use crate::store::store_partial_u64;

pub fn tv_ltu64_division(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    assert!(size.get() <= 64);
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.checked_div(r).unwrap_or(0); // Division by zero, returns 0
    store_partial_u64(dst, out, size);
}
pub fn tv_ltu64_modulus(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
    assert!(size.get() <= 64);
    let l = load_partial_u64(&lhs, size);
    let r = load_partial_u64(&rhs, size);
    let out = l.checked_rem(r).unwrap_or(0); // Division by zero, returns 0
    store_partial_u64(dst, out, size);
}

/// Two-value logic arbitary precision division.
pub fn tv_division(
    quotient: &mut [u64],
    modulus: &mut [u64],
    numerator: &[u64],
    denumerator: &[u64],
    size: VectorSize,
) {
    assert!(
        quotient.len() > 0
            && quotient.len() == modulus.len()
            && quotient.len() == numerator.len()
            && quotient.len() == denumerator.len()
            && quotient.len() == size.get().div_ceil(64) as usize
    );

    // Fast path: size <= 64
    if quotient.len() == 1 && denumerator[0] != 0 {
        quotient[0] = numerator[0] / denumerator[0];
        modulus[0] = numerator[0] % denumerator[0];
        return;
    }

    quotient.fill(0u64);
    modulus.copy_from_slice(numerator);

    let denum_lz = tv_leading_zeros(denumerator, size);
    if denum_lz == size.get() {
        modulus.fill(0u64); // Division by zero, quotient and modulus are zero.
        return;
    }
    while tv_gtu64_unsigned_leq(denumerator, modulus, size) {
        let mod_lz = tv_leading_zeros(modulus, size);
        let offset = denum_lz - mod_lz;

        // (rb << offset) > modulus (computed as !((rb << offset) <= modulus))
        let shift_one_less = !tv_lsl_unsigned_leq(denumerator, offset, modulus, size);

        // If (modulus << offset) > denumerator, we need to move one further.
        let offset = offset - u32::from(shift_one_less);

        // quotient |= 1 << offset
        quotient[(offset / 64) as usize] |= 1u64 << (offset % 64);

        // modulus -= denum << offset;
        tv_lsl_mut_sub(modulus, denumerator, offset, size);
    }
}

/// Computes `dst_lhs -= rhs << offset`.
pub fn tv_lsl_mut_sub(dst_lhs: &mut [u64], rhs: &[u64], offset: u32, size: VectorSize) {
    assert!(dst_lhs.len() > 0 && dst_lhs.len() == rhs.len());
    let nwords = dst_lhs.len();
    let mut carry_in = true;
    let soff = offset % 64;
    if soff == 0 {
        let swords = offset as usize / 64;
        for i in 0..swords {
            (dst_lhs[i], carry_in) = dst_lhs[i].carrying_add(!0u64, carry_in);
        }
        for i in 0..nwords - swords {
            (dst_lhs[i + swords], carry_in) = dst_lhs[i + swords].carrying_add(!rhs[i], carry_in);
        }
    } else {
        let swords = offset.div_ceil(64) as usize;
        for i in 0..swords - 1 {
            (dst_lhs[i], carry_in) = dst_lhs[i].carrying_add(!0u64, carry_in);
        }
        (dst_lhs[swords - 1], carry_in) =
            dst_lhs[swords - 1].carrying_add(!(rhs[0] << soff), carry_in);
        for i in 0..nwords - swords {
            let value = (rhs[i + 1] << soff) | (rhs[i] >> (64 - soff));
            (dst_lhs[i + swords], carry_in) = dst_lhs[i + swords].carrying_add(!value, carry_in);
        }
    }
    if size.get() % 64 != 0 {
        *dst_lhs.last_mut().unwrap() &= (1u64 << (size.get() % 64)).wrapping_sub(1);
    }
}

/// Computes `(lhs << shift) <= rhs`.
///
/// This is used in the long division kernel.
///
/// # Invariant:
/// - `lhs.len() == rhs.len()`
pub fn tv_lsl_unsigned_leq(lhs: &[u64], shift: u32, rhs: &[u64], size: VectorSize) -> bool {
    let nwords = size.get().div_ceil(64) as usize;
    assert!(nwords == lhs.len() && nwords == rhs.len());

    if shift == 0 {
        return tv_gtu64_unsigned_leq(lhs, rhs, size);
    }
    if shift >= size.get() {
        return true;
    }
    let shift = shift as usize;
    let soff = shift % 64;
    let swords = shift.div_ceil(64);
    if soff == 0 {
        let mut l = lhs[nwords - swords - 1];
        if size.get() % 64 != 0 {
            l &= (1u64 << (size.get() % 64)).wrapping_sub(1);
        }
        let r = rhs[nwords - 1];
        match l.cmp(&r) {
            Ordering::Less => return true,
            Ordering::Greater => return false,
            Ordering::Equal => {}
        }

        for i in (0..nwords - swords - 1).rev() {
            let value = match lhs[i].cmp(&rhs[i + swords]) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => continue,
            };
            return value;
        }
        true
    } else {
        let mut bs = size.get();
        for i in (0..nwords - swords).rev() {
            let mut l = (lhs[i + 1] << soff) | (lhs[i] >> (64 - soff));
            let r = rhs[i + swords];

            let nbs = bs % 64;
            if nbs != 0 {
                l &= (1u64 << nbs).wrapping_sub(1);
                bs -= nbs;
            } else {
                bs -= 64;
            }

            let value = match l.cmp(&r) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => continue,
            };
            return value;
        }
        let mut l = lhs[0] << soff;
        let nbs = bs % 64;
        if nbs != 0 {
            l &= (1u64 << nbs).wrapping_sub(1);
        }
        let r = rhs[swords - 1];
        l <= r
    }
}

#[cfg(test)]
mod tests {
    use super::{tv_division, tv_lsl_mut_sub, tv_lsl_unsigned_leq};
    use crate::VectorSize;
    use crate::arithmetic::tests::{
        u64x2_to_slice, u64x2_to_slice_mut, u128_arith_target, u128_to_u64x2,
    };
    use crate::arithmetic::tv_subtraction;
    use crate::comparison::tv_gtu64_unsigned_leq;
    use crate::proptest::{any_bits_of_size, any_reasonable_size};
    use crate::shift::tv_gtu64_logical_shift_left;
    use proptest::prelude::Just;
    use proptest::proptest;

    proptest::prop_compose! {
        fn lsl_unsigned_leq_target
            ()
            (size in any_reasonable_size(1..=2048))
            (size in Just(size), lhs in any_bits_of_size(size), rhs in any_bits_of_size(size), shift in 0..=size.get())
                -> (VectorSize, Vec<u64>, Vec<u64>, u32) {
                (size, lhs, rhs, shift)
        }
    }

    proptest! {
        #[test]
        fn proptest_tv_lsl_unsigned_leq
            ((size, lhs, rhs, shift) in lsl_unsigned_leq_target())
        {
            let nwords = size.get().div_ceil(64) as usize;
            let mut reference = vec![0u64; nwords];

            tv_gtu64_logical_shift_left(&mut reference, &lhs, shift, size);
            let reference = tv_gtu64_unsigned_leq(&reference, &rhs, size);
            let target = tv_lsl_unsigned_leq(&lhs, shift, &rhs, size);

            proptest::prop_assert_eq!(reference, target);
        }
    }

    proptest! {
        #[test]
        fn proptest_tv_division
            ((size, lhs, rhs) in u128_arith_target())
        {
            let mask = (1u128.unbounded_shl(size.get())).wrapping_sub(1);
            let expected_quotient = if rhs == 0 { [0, 0] } else { u128_to_u64x2((lhs / rhs) & mask) };
            let expected_modulus = if rhs == 0 { [0, 0] } else { u128_to_u64x2((lhs % rhs) & mask) };
            let mut given_quotient = [0u64; 2];
            let mut given_modulus = [0u64; 2];

            tv_division(
                u64x2_to_slice_mut(&mut given_quotient, size),
                u64x2_to_slice_mut(&mut given_modulus, size),
                u64x2_to_slice(&u128_to_u64x2(lhs), size),
                u64x2_to_slice(&u128_to_u64x2(rhs), size),
                size
            );

            proptest::prop_assert_eq!(given_quotient, expected_quotient);
            proptest::prop_assert_eq!(given_modulus, expected_modulus);
        }
    }

    proptest::prop_compose! {
        fn lsl_mut_sub_target
            ()
            (size in any_reasonable_size(1..=2048))
            (size in Just(size), lhs in any_bits_of_size(size), rhs in any_bits_of_size(size), shift in 0..=size.get())
                -> (VectorSize, Vec<u64>, Vec<u64>, u32) {
                (size, lhs, rhs, shift)
        }
    }

    proptest! {
        #[test]
        fn proptest_tv_lsl_mut_sub
            ((size, mut lhs, rhs, shift) in lsl_mut_sub_target())
        {
            let nwords = size.get().div_ceil(64) as usize;
            let mut reference_shift = vec![0u64; nwords];
            let mut reference_sub = vec![0u64; nwords];

            tv_gtu64_logical_shift_left(&mut reference_shift, &rhs, shift, size);
            tv_subtraction(&mut reference_sub, &lhs, &reference_shift, size);
            tv_lsl_mut_sub(&mut lhs, &rhs, shift, size);

            proptest::prop_assert_eq!(&reference_sub, &lhs);
        }
    }

    #[test]
    fn test_division_vectors() {
        macro_rules! assert_test_vector {
            ($size:literal, $a:expr, $b:expr, $q:expr, $m:expr) => {
                let mut quotient = [0u64; $size.div_ceil(64) as usize];
                let mut modulus = [0u64; $size.div_ceil(64) as usize];
                tv_division(
                    &mut quotient,
                    &mut modulus,
                    $a,
                    $b,
                    VectorSize::new($size).unwrap(),
                );
                assert_eq!(quotient.as_slice(), $q.as_slice(), "quotient");
                assert_eq!(modulus.as_slice(), $m.as_slice(), "modulus");
            };
        }

        assert_test_vector!(1u32, &[0u64], &[0u64], &[0u64], &[0u64]);
        assert_test_vector!(1u32, &[1u64], &[0u64], &[0u64], &[0u64]);
        assert_test_vector!(1u32, &[1u64], &[1u64], &[1u64], &[0u64]);
        assert_test_vector!(2u32, &[1u64], &[1u64], &[1u64], &[0u64]);
        assert_test_vector!(8u32, &[64u64], &[8u64], &[8u64], &[0u64]);
        assert_test_vector!(8u32, &[64u64], &[7u64], &[9u64], &[1u64]);
        assert_test_vector!(
            128u32,
            &[0x0000000000000000u64, 0x0000000000000001u64],
            &[0x0000000000000003u64, 0x0000000000000000u64],
            &[0x5555555555555555u64, 0x0000000000000000u64],
            &[0x0000000000000001u64, 0x0000000000000000u64]
        );
        assert_test_vector!(
            128u32,
            &[0x0A3254CF8CEF6099u64, 0x77C4CD994D0447C2u64],
            &[0x71B4D413F917E7BAu64, 0x0000000E37A5AA69u64],
            &[0x00000000086C92BFu64, 0x0000000000000000u64],
            &[0xFB1EE0857F7968D3u64, 0x0000000A1D306152u64]
        );
        assert_test_vector!(
            2048u32,
            &[
                0xA31AFB7131E2412Fu64,
                0xD75E50B5A34C41D3u64,
                0x806451A82898D777u64,
                0xBF67DF31E66A5F2Du64,
                0xEE2A274B72199611u64,
                0x4EDA71A5454DF367u64,
                0xC29D3E8E6BCC5513u64,
                0x79F78131196A2FC7u64,
                0x563AE7B26BD500B3u64,
                0x54AEE63DF83E85F6u64,
                0x1B088AF443A213EDu64,
                0xBCE563C9898389B5u64,
                0xE828E3EEDDC7BDA9u64,
                0xFFF34BAD5EFC531Eu64,
                0x32331C77BF820B21u64,
                0x89E92A5E6111D73Fu64,
                0x2B88BFE075AA4CE9u64,
                0xD9E6118EBBC46193u64,
                0x458395175509E8C1u64,
                0xC18D65B56B74A87Au64,
                0x6F1BE59D6E65B909u64,
                0xB572C990C517EEE1u64,
                0xC978F770C2B591D2u64,
                0x3486A7972CCEFF3Au64,
                0x0948DD33860586B7u64,
                0x02B0F076467C7264u64,
                0x362775160D96E6A8u64,
                0xCBD52C69AD99BCFAu64,
                0x38DB2E4AEC77C8F5u64,
                0x0AA4CD106C4BD342u64,
                0x7AD14D907A008A0Du64,
                0x6FA754B30F7A15ACu64
            ],
            &[
                0x9F41FB337A385226u64,
                0x6D9F84CDCE3700C9u64,
                0x20DDDB480100E7D6u64,
                0xF25A646A3BFF656Fu64,
                0xCB7294E6C3CBEBDDu64,
                0x0F3FDC5F2C251D0Eu64,
                0x49EA8A5996B8C40Bu64,
                0x08FA18911AA6193Eu64,
                0x8EA13308CD039E02u64,
                0x10E674BAA0F68681u64,
                0xD95BC0CF8282A0C2u64,
                0x93E32CDFEA6F3BB0u64,
                0x54C58DEC9CC8AFD4u64,
                0xE0D8B7BAB45DAEC0u64,
                0x0187F4FF7F10C9BDu64,
                0xC0BA594EB6FD01D9u64,
                0x816514764224E87Au64,
                0xAC7F777ECFC69AA5u64,
                0xCE0EF649F48FEA98u64,
                0xDB35465635F081E4u64,
                0xA7FDE4D2E7B80AA0u64,
                0x8E8E64E20114A319u64,
                0x0FD664FFD1AE0591u64,
                0x9088DC8C391E7CD6u64,
                0x9056AF326BBCFB91u64,
                0x43247332AD12F2B9u64,
                0x9D2C6BFDDCC182D6u64,
                0x31EE72BD7298944Du64,
                0x1CAA05E655965D43u64,
                0x38F4E92A5266CCF6u64,
                0xE77689BC3F63FB27u64,
                0x000000000000CE00u64
            ],
            &[
                0x00008AC06C6C575Bu64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64
            ],
            &[
                0xE27AC6B0EBEA23ADu64,
                0x8008AFDE3D5548BAu64,
                0x4B8DA09E048C7826u64,
                0x06CB0970DBCFCFDBu64,
                0xB9C9A015E9A75C4Cu64,
                0xF2FCB7211CF65A3Au64,
                0xF9971751D1B4EDDFu64,
                0x3013A24561D8DBC3u64,
                0xE67E27583F6333DCu64,
                0x690D5A65B3A97BC8u64,
                0x79A9223FE1CC79FDu64,
                0xC7EC699B21A157F0u64,
                0x2B70C18BB20B67C1u64,
                0x5F272DD27BB8BEC0u64,
                0xF3DFD7A833701307u64,
                0x30295E1A9545F4C0u64,
                0xCBDA47C18A8A9BCFu64,
                0x6981D9F5C85CD624u64,
                0xD5B8C38305F51A90u64,
                0x952F1DC9935FE699u64,
                0xEC99CCF60C27197Au64,
                0xCD58CD8BE1DCF1EEu64,
                0xB2099C77A73B6AEAu64,
                0xD7829FB9165117CDu64,
                0x68DC20B8486DEB1Cu64,
                0xC222AF41A5A11000u64,
                0xB4C56619392F2864u64,
                0x7C6E3940F461C146u64,
                0x5816183CB838F53Eu64,
                0x20240324BF7AEAEDu64,
                0xF456EFD9B928E4E7u64,
                0x0000000000001D37u64
            ]
        );
        assert_test_vector!(
            2041u32,
            &[
                0x73F1C29A70A80A21u64,
                0x1DCB1DF928EC1BF7u64,
                0x57208290E21785DFu64,
                0x1C04841EAE8116C9u64,
                0xA103E18B4778BA06u64,
                0xAF63FCC05DA1F8A0u64,
                0x84C046D6BF96A3B5u64,
                0xEE2463DB162809CAu64,
                0xA7477B00A700BFF8u64,
                0xA132351B05545873u64,
                0xA8AA28CEE0FD9047u64,
                0xA8A653A05B93E0DDu64,
                0xB6648DCBDB0869C0u64,
                0x7DDF41A825FA5733u64,
                0x87AE33E5D0593EA8u64,
                0x09F7564DB76BB02Du64,
                0xDC0F06002DB8775Cu64,
                0x15CC3E772237CEFBu64,
                0xC6A4D3BAC9D81F2Du64,
                0x1008F8AD961BF2CFu64,
                0xDCB670C9D5886CFFu64,
                0x653C55DA22C74CF8u64,
                0x6F1446B8BCDC614Du64,
                0xA5F8EEC91058D8B6u64,
                0xB897E2783BCC78B9u64,
                0x46930185398D5D10u64,
                0x73976450868544D3u64,
                0x7D589409DE8A9042u64,
                0x2BBCA6F6C548DF09u64,
                0x40510FECD858D091u64,
                0x2D53F9DC9752E92Fu64,
                0x016A3BD0CA61AB2Cu64
            ],
            &[
                0x085FB71F880BF2E9u64,
                0x883453B4AB9A666Eu64,
                0xC83C543824F0B2CFu64,
                0x51486114AE37D441u64,
                0x942B9DA4EAD4F78Au64,
                0x1D511335CAF6D966u64,
                0x3D5B4728D0DDE637u64,
                0x1C8A254C3DDA102Eu64,
                0xFA1111767E34D5F7u64,
                0x4F4894091858A506u64,
                0xC4739B57DBBA4620u64,
                0x6394A2C4C7438698u64,
                0x133EBF06A47C9EF6u64,
                0x745A9BC4C249609Bu64,
                0xAA661731F9F84033u64,
                0x40707E618C764CACu64,
                0x053B5D9AE3DFD339u64,
                0x072E58E434C43C50u64,
                0x90AD0DDEF7AEAAE6u64,
                0x730B27F66C396EBEu64,
                0xEE65CEA2B3BE0B93u64,
                0xAC263520982551BFu64,
                0xB11098D11FD19B77u64,
                0x81FF737D6A8F4216u64,
                0x43EE7F8D0A702E78u64,
                0x096DBCFD9874E74Eu64,
                0x352AA389A6CB2E24u64,
                0x6D83F67944EE2DE8u64,
                0xFF1393A7BD81DAA1u64,
                0x601F6AF4FBDFB75Fu64,
                0x002974AB50315892u64,
                0x0000000000000000u64
            ],
            &[
                0xBCE3253281B8FE32u64,
                0x0000000000000008u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64,
                0x0000000000000000u64
            ],
            &[
                0x8A4AC8F602D76A9Fu64,
                0xAA8604CB206D5B02u64,
                0x5446C71B528035BFu64,
                0x590F3B40A96E5FEEu64,
                0xBE3DC2A570B42EC3u64,
                0x8CC9F6D860AD44B0u64,
                0xDD2FE074395E7E51u64,
                0x3E589D56F0BBBFB5u64,
                0x3D37BD2FBC2EDBC9u64,
                0x4FCF0B226DFDEBD2u64,
                0x6F6D424AC7922E3Cu64,
                0x3B4ABA8F81EDB324u64,
                0xE7606EA0AC36B63Fu64,
                0x0F6798D696318C72u64,
                0x0C2CC49618E5522Eu64,
                0x331692BFAC55C18Cu64,
                0x6596423CFE39CCBFu64,
                0xBC3D247EFB1741C3u64,
                0x2E820C4D3413DE05u64,
                0x85B2F659CA46E1C1u64,
                0x10EBABE0AE3E88EAu64,
                0x416B237650571245u64,
                0x89BB6B248FF75569u64,
                0x477300CB1E94B872u64,
                0x0CE407F29D223A3Cu64,
                0x8D87227B067C2343u64,
                0x3D5B883AC02C98C9u64,
                0x1FB1756438EC1DEFu64,
                0xA8B3FC8234CBA7FCu64,
                0x61CD7951D30C3C5Cu64,
                0x00209B319C5FCC74u64,
                0x0000000000000000u64
            ]
        );
    }
}
