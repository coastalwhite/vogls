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

pub const fn tv_posedge(before: bool, after: bool) -> bool {
    after & !before
}
pub const fn tv_negedge(before: bool, after: bool) -> bool {
    before & !after
}
