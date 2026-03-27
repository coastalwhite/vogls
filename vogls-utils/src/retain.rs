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
