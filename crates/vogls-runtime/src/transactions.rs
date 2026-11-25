use crate::RtSignalKey;

/// A container to track which signals have received updates.
///
/// The container consists of a vector split into two slices:
/// - Commited: sorted deduplicated slice
/// - Uncommited: unsorted non-deduplicated slice
///
/// Every time the vector needs to grow, the end slice is sorted and the two slices are merged and
/// deduplicated. This balances memory consumption and cheap amortized performance.
#[derive(Default, Clone)]
pub struct Transactions {
    txs: Vec<RtSignalKey>,

    /// The split point of the commited slice and the uncommited slice.
    checkpoint: usize,
}

impl Transactions {
    pub const fn new() -> Self {
        Self {
            txs: Vec::new(),
            checkpoint: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, value: RtSignalKey) {
        // We don't have to grow the transactions vector, just keep pushing.
        if self.txs.len() < self.txs.capacity() {
            self.txs.push(value);
            return;
        }

        std::hint::cold_path();
        self.flush();

        // @NOTE: Keep at least as many free slots as there are unique transactions, so the next
        // flush is at least `len` pushes away. Otherwise a buffer that is nearly full of unique
        // transactions would come back here on almost every push.
        self.txs.reserve(self.txs.len());
        self.txs.push(value);
    }

    pub fn flush(&mut self) {
        if self.checkpoint == self.txs.len() {
            return;
        }

        self.txs.sort_unstable();
        self.txs.dedup();
        self.checkpoint = self.txs.len();
    }

    pub fn flush_as_slice(&mut self) -> &[RtSignalKey] {
        self.flush();
        &self.txs
    }

    pub fn reserve(&mut self, additional: usize) {
        self.txs.reserve(additional);
    }

    pub fn clear(&mut self) {
        self.txs.clear();
        self.checkpoint = 0;
    }

    /// The buffer, the checkpoint and the scratch space.
    pub fn into_inner(self) -> (Vec<RtSignalKey>, usize) {
        (self.txs, self.checkpoint)
    }
    pub fn from_inner(txs: Vec<RtSignalKey>, checkpoint: usize) -> Self {
        assert!(checkpoint <= txs.len());
        debug_assert!(txs[..checkpoint].is_sorted());
        debug_assert!(txs[..checkpoint].array_windows::<2>().all(|[a, b]| a != b));
        Self { txs, checkpoint }
    }
}
