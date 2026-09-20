use vogls_bits::arithmetic::FvLogicValue;
use vogls_bits::edge::{fv_negedge, fv_posedge};

use vogls_ir::{SCALAR_VSIZE, VectorSize, WatchCondition, WatchEdge};

use crate::{BytecodeListeners, Schedule};

#[derive(Default, Clone, Copy)]
pub struct BytecodeWatcher {
    pub idx: u64,
    pub meta: BytecodeWatcherMeta,
}

#[derive(Default, Clone, Copy)]
pub struct BytecodeWatcherMeta(u64);

impl BytecodeWatcherMeta {
    /// Pack a watch condition.
    ///
    /// The offset is the one the condition carries: a bit position within the watched signal, not
    /// a heap address. The watcher list is already looked up per signal, so the signal's identity
    /// is not the offset's job to carry -- and keeping it relative means the packing is bounded by
    /// how wide one signal is rather than by how large the whole heap is, and holds the same
    /// number Cranelift keeps on its `Listener`.
    pub fn new(condition: WatchCondition) -> Self {
        let offset = condition.offset as u64;
        if offset >= Self::MAX_OFFSET {
            // @NOTE: Past what the packing holds, an undirected watch falls back to covering
            // everything: it then wakes on writes it does not care about, which costs time and no
            // correctness. A directed one has nowhere to degrade to -- dropping the direction
            // would make the process trigger on the wrong edge -- so it has to be an error.
            assert!(
                matches!(condition.edge, WatchEdge::Any { .. }),
                "Edge watch at bit {offset}, past the {} the watcher packing holds",
                Self::MAX_OFFSET
            );
            return Self((Self::MAX_OFFSET - 1) << 33);
        }
        Self(match condition.edge {
            WatchEdge::Posedge => Edge::Posedge as u64 | (offset << 2),
            WatchEdge::Negedge => Edge::Negedge as u64 | (offset << 2),
            WatchEdge::Any { bit_length } => (offset << 2) | ((bit_length.get() as u64) << 33),
        })
    }

    /// One past the highest bit offset within a signal the packed 31-bit offset field can name.
    pub const MAX_OFFSET: u64 = 1 << 31;

    /// Does this watch ask for one direction of transition, rather than any change?
    pub fn is_directed(self) -> bool {
        self.0 & 1 != 0
    }

    fn start(self) -> u32 {
        ((self.0 >> 2) & 0x7FFF_FFFF) as u32
    }

    /// The number of bits this watch covers. An edge watch is on a scalar by construction, and
    /// stores no width of its own.
    fn width(self) -> u32 {
        if self.is_directed() {
            1
        } else {
            (self.0 >> 33) as u32
        }
    }

    /// Does this watch ask for `edge`? Only meaningful once it is known to be directed.
    fn wants(self, edge: Edge) -> bool {
        self.0 & 0x3 == edge as u64
    }

    /// Does a one-bit write at `offset` that took `edge` trigger this watch?
    pub fn satify1(self, offset: u32, edge: Edge) -> bool {
        if !self.is_directed() {
            // @NOTE: A watch with no direction covers a range of bits, and a one-bit write into
            // any of them triggers it -- it need not be the bit the range starts at.
            return self.overlaps(offset, SCALAR_VSIZE);
        }
        (self.start() == offset) & self.wants(edge)
    }

    /// Does a write of `bit_length` bits at `offset` overlap the bits this watch covers?
    ///
    /// An edge watch is woken by any overlapping wide write: only a one-bit write knows which
    /// direction the signal moved, so anything wider has to fall back to waking unconditionally.
    pub fn overlaps(self, offset: u32, bit_length: VectorSize) -> bool {
        // @NOTE: In u64 so that a watch at the top of the offset range plus a wide signal cannot
        // carry past u32 and wrap the comparison around.
        let start = self.start() as u64;
        let end = start + self.width() as u64;
        let offset = offset as u64;
        (offset < end) & (offset + bit_length.get() as u64 > start)
    }
}

/// Which way a one-bit signal moved, or [`Edge::Any`] for a transition that is neither a posedge
/// nor a negedge (`x` to `z`, say) and so triggers only the watches that ask for no direction.
///
/// Bit 0 marks "this is a directed edge" and bit 1 carries the direction, which is the same
/// layout [`BytecodeWatcherMeta`] stores in its low two bits.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    #[default]
    Any = 0b00,
    Negedge = 0b01,
    Posedge = 0b11,
}

impl Edge {
    /// The edge a one-bit signal takes moving from `old` to `new`.
    #[inline(always)]
    pub fn between(old: FvLogicValue, new: FvLogicValue) -> Self {
        if fv_posedge(old, new) {
            Edge::Posedge
        } else if fv_negedge(old, new) {
            Edge::Negedge
        } else {
            Edge::Any
        }
    }

    /// The edge a one-bit two-valued signal takes on settling at `new`.
    ///
    /// Only called once the write is known to have changed the bit, so `new` is the whole story:
    /// the previous level was its inverse.
    #[inline(always)]
    pub fn from_level(new: bool) -> Self {
        if new { Edge::Posedge } else { Edge::Negedge }
    }
}

pub struct BytecodeWatchers {
    pub offsets: Vec<u64>,
    pub watchers: Vec<BytecodeWatcher>,
}

impl BytecodeWatcher {
    pub fn from_condition(idx: u64, condition: WatchCondition) -> Self {
        Self {
            idx,
            meta: BytecodeWatcherMeta::new(condition),
        }
    }
}

impl BytecodeWatchers {
    #[inline(always)]
    pub fn get(&self, index: usize) -> &[BytecodeWatcher] {
        let end = index.saturating_add(1);
        assert!(end < self.offsets.len());
        let start = self.offsets[index];
        let end = self.offsets[index.saturating_add(1)];
        &self.watchers[start as usize..end as usize]
    }
}

/// Wake `watcher` if a write of `bit_length` bits at bit `offset` of the signal touches the bits
/// it watches.
///
/// `change` is the write's update mask and the value it settled on, both relative to `offset`. A
/// directed watch needs them: a wide write reaches here when *any* of its bits changed, which says
/// nothing about the one bit being watched. Where a caller cannot produce them -- a four-valued or
/// wider-than-64-bit write -- it passes `None`, and a directed watch on such a signal is a bug,
/// since there is no way left to tell one edge from the other.
#[inline(always)]
pub fn wake(
    watcher: BytecodeWatcher,
    offset: u32,
    bit_length: VectorSize,
    change: Option<(u64, u64)>,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
) {
    if !watcher.meta.overlaps(offset, bit_length) {
        return;
    }

    if watcher.meta.is_directed() {
        let Some((update, value)) = change else {
            debug_assert!(
                false,
                "A write of {bit_length:?} bits reached a directed edge watch"
            );
            return wake_all(watcher, schedule, listeners);
        };
        // @NOTE: `overlaps` put the watched bit inside the write, and a directed watch is one bit
        // wide, so this shift stays under the write's own width.
        let bit = watcher.meta.start() - offset;
        if (update >> bit) & 1 == 0 {
            return;
        }
        if !watcher
            .meta
            .wants(Edge::from_level((value >> bit) & 1 != 0))
        {
            return;
        }
    }

    wake_all(watcher, schedule, listeners);
}

/// Wake `watcher` if a one-bit write at bit `offset` of the signal that took `edge` satisfies it.
#[inline(always)]
pub fn wake1(
    watcher: BytecodeWatcher,
    edge: Edge,
    offset: u32,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
) {
    if !watcher.meta.satify1(offset, edge) {
        return;
    }

    wake_all(watcher, schedule, listeners);
}

/// Wake `watcher` whatever it watches for.
#[inline(always)]
pub fn wake_all(
    watcher: BytecodeWatcher,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
) {
    let index = watcher.idx as usize;
    let bit = 1u64 << (index % 64);
    let is_listening = (listeners.active[index / 64] & bit) != 0;
    if is_listening {
        let offset = listeners.map[index];
        schedule.active.push(offset);
        listeners.active[index / 64] ^= bit;
    }
}
