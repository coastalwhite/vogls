use crate::arithmetic::FvLogicValue;

pub const fn fv_posedge(before: FvLogicValue, after: FvLogicValue) -> bool {
    use FvLogicValue as L;
    matches!(
        (before, after),
        (L::L0, L::L1 | L::X | L::Z) | (L::X | L::Z, L::L1)
    )
}

pub const fn fv_negedge(before: FvLogicValue, after: FvLogicValue) -> bool {
    use FvLogicValue as L;
    matches!(
        (before, after),
        (L::L1, L::L0 | L::X | L::Z) | (L::X | L::Z, L::L0)
    )
}

pub const fn fv_posedge_u64(xspc: u64, xval: u64, yspc: u64, yval: u64) -> u64 {
    (xspc & !xval & (!yspc | yval)) | (!xspc & yspc & yval)
}

pub const fn fv_negedge_u64(xspc: u64, xval: u64, yspc: u64, yval: u64) -> u64 {
    (xspc & xval & (!yspc | !yval)) | (!xspc & yspc & !yval)
}

pub const fn tv_posedge(before: bool, after: bool) -> bool {
    after & !before
}
pub const fn tv_negedge(before: bool, after: bool) -> bool {
    before & !after
}

#[cfg(test)]
mod tests {
    use super::*;
    use FvLogicValue as L;
    use FvLogicValue::*;

    #[rustfmt::skip]
    const TEST_VECTORS: [(L, L, bool, bool); 16] = [
        //  lhs rhs       POSEDGE    NEGEDGE
        (   X,  X,        false,     false,    ),
        (   X,  Z,        false,     false,    ),
        (   X,  L0,       false,     true,     ),
        (   X,  L1,       true,      false,    ),

        (   Z,  X,        false,     false,    ),
        (   Z,  Z,        false,     false,    ),
        (   Z,  L0,       false,     true,     ),
        (   Z,  L1,       true,      false,    ),

        (   L0, X,        true,      false,    ),
        (   L0, Z,        true,      false,    ),
        (   L0, L0,       false,     false,    ),
        (   L0, L1,       true,      false,    ),

        (   L1, X,        false,     true,     ),
        (   L1, Z,        false,     true,     ),
        (   L1, L0,       false,     true,     ),
        (   L1, L1,       false,     false,    ),
    ];

    #[test]
    fn test_fv_edges() {
        for (x, y, expect_posedge, expect_negedge) in TEST_VECTORS {
            let (xspc, xval) = (x.spc() as u64, x.val() as u64);
            let (yspc, yval) = (y.spc() as u64, y.val() as u64);

            let posedge = fv_posedge_u64(xspc, xval, yspc, yval) == 1;
            let negedge = fv_negedge_u64(xspc, xval, yspc, yval) == 1;

            assert_eq!(
                posedge, expect_posedge,
                "posedge({x:?}, {y:?}), expected = {expect_posedge}, gotten = {posedge}"
            );
            assert_eq!(
                negedge, expect_negedge,
                "negedge({x:?}, {y:?}), expected = {expect_negedge}, gotten = {negedge}"
            );
        }
    }
}
