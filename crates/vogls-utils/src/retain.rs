pub fn slice_retain<T: Copy>(items: &mut [T], mut f: impl FnMut(&T) -> bool) -> usize {
    let Some(fst) = items.iter().position(|i| !f(i)) else {
        return items.len();
    };
    let mut write = fst;
    for read in fst + 1..items.len() {
        if f(&items[read]) {
            items[write] = items[read];
            write += 1;
        }
    }
    write
}

pub fn retain_vec_end<T: Copy>(items: &mut Vec<T>, offset: usize, mut f: impl FnMut(T) -> bool) {
    if items.len() <= offset {
        return;
    }

    let mut w = offset;
    for r in offset..items.len() {
        let t = items[r];
        if f(t) {
            // SAFETY: This is true as `r` starts at the same value as `w` and is incremented on
            // every loop condition.
            unsafe { std::hint::assert_unchecked(w <= r) };

            items[w] = t;
            w += 1;
        }
    }
    items.truncate(w);
}
