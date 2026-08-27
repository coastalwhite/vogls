pub fn select_merge(lspc: u64, lval: u64, rspc: u64, rval: u64) -> (u64, u64) {
    let spc = lspc & rspc & !(lval ^ rval);
    let val = lval & spc;
    (spc, val)
}
