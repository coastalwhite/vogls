use std::fmt;

use vogls_ir::Time;
use vogls_verilog::lower::specify::{Delay, Delays};

/// A pointer to a certain amount of delay values.
///
/// This is a bit-packed structure that contains an offset (49 bits), triple mask (12 bits) and
/// variant (3 bits).
///
/// - The offset is a element offset into a vector.
/// - The triple mask is a bitmask that determines whether an element stores a `min:typ:max` (when
///   set to 1) or just a single `typ` value (when set to 0).
/// - The variant determines whether there are 1, 2, 3, 6 or 12 elements. The encoding is of the
/// the values of [`DelayPtrVariant`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DelayPtr(u64);

impl fmt::Debug for DelayPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DelayPtr")
            .field("offset", &self.offset())
            .field("triple_mask", &self.triple_mask())
            .field("variant", &self.variant())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DelayPtrVariant {
    One = 0,
    Two = 1,
    Three = 2,
    Six = 3,
    Twelve = 4,
}

impl DelayPtr {
    pub fn new(offset: u64, variant: DelayPtrVariant, triple_mask: u16) -> Self {
        debug_assert_eq!(offset & 0x1_FFFF_FFFF_FFFF, offset);
        debug_assert_eq!(triple_mask & ((1u16 << variant.len()) - 1), triple_mask);

        let offset = offset & 0x1_FFFF_FFFF_FFFF;
        let triple_mask = triple_mask & ((1u16 << variant.len()) - 1);

        Self((offset << 15) | ((triple_mask as u64) << 3) | (variant as u64))
    }

    #[inline(always)]
    pub fn num_triples(self) -> u32 {
        self.triple_mask().count_ones()
    }
    #[inline(always)]
    pub fn num_delays(self) -> usize {
        self.variant().len()
    }

    pub fn total_num_entries(self) -> usize {
        self.num_triples() as usize * 2 + self.num_delays()
    }

    pub fn offset(self) -> u64 {
        self.0 >> 15
    }

    pub fn variant(self) -> DelayPtrVariant {
        use DelayPtrVariant as V;
        match self.0 & 0x7 {
            0 => V::One,
            1 => V::Two,
            2 => V::Three,
            3 => V::Six,
            4 => V::Twelve,
            _ => unreachable!(),
        }
    }

    pub fn triple_mask(self) -> u16 {
        ((self.0 >> 3) & 0xFFF) as u16
    }

    // @TODO: Remove and internalize DelayPtr into timing lowering.
    pub fn materialize(self, delays: &[Time]) -> Delays {
        use DelayPtrVariant as V;
        let mut ptr = self.offset();
        let mut triple_mask = self.triple_mask();
        match self.variant() {
            V::One => Delays::One(read_one(delays, &mut ptr, &mut triple_mask)),
            V::Two => Delays::Two {
                trise: read_one(delays, &mut ptr, &mut triple_mask),
                tfall: read_one(delays, &mut ptr, &mut triple_mask),
            },
            V::Three => Delays::Three {
                trise: read_one(delays, &mut ptr, &mut triple_mask),
                tfall: read_one(delays, &mut ptr, &mut triple_mask),
                tz: read_one(delays, &mut ptr, &mut triple_mask),
            },
            V::Six => Delays::Six {
                t01: read_one(delays, &mut ptr, &mut triple_mask),
                t10: read_one(delays, &mut ptr, &mut triple_mask),
                t0z: read_one(delays, &mut ptr, &mut triple_mask),
                tz1: read_one(delays, &mut ptr, &mut triple_mask),
                t1z: read_one(delays, &mut ptr, &mut triple_mask),
                tz0: read_one(delays, &mut ptr, &mut triple_mask),
            },
            V::Twelve => Delays::Twelve {
                t01: read_one(delays, &mut ptr, &mut triple_mask),
                t10: read_one(delays, &mut ptr, &mut triple_mask),
                t0z: read_one(delays, &mut ptr, &mut triple_mask),
                tz1: read_one(delays, &mut ptr, &mut triple_mask),
                t1z: read_one(delays, &mut ptr, &mut triple_mask),
                tz0: read_one(delays, &mut ptr, &mut triple_mask),
                t0x: read_one(delays, &mut ptr, &mut triple_mask),
                tx1: read_one(delays, &mut ptr, &mut triple_mask),
                t1x: read_one(delays, &mut ptr, &mut triple_mask),
                tx0: read_one(delays, &mut ptr, &mut triple_mask),
                txz: read_one(delays, &mut ptr, &mut triple_mask),
                tzx: read_one(delays, &mut ptr, &mut triple_mask),
            },
        }
    }
}

pub fn read_one(delays: &[Time], ptr: &mut u64, triple_mask: &mut u16) -> Delay {
    let is_triple = *triple_mask & 1 != 0;
    *triple_mask >>= 1;

    if is_triple {
        let min = delays[*ptr as usize].0;
        *ptr += 1;
        let typ = delays[*ptr as usize].0;
        *ptr += 1;
        let max = delays[*ptr as usize].0;
        *ptr += 1;
        Delay { min, typ, max }
    } else {
        let typ = delays[*ptr as usize].0;
        *ptr += 1;
        Delay {
            min: typ,
            typ,
            max: typ,
        }
    }
}

impl DelayPtrVariant {
    pub fn len(self) -> usize {
        match self {
            DelayPtrVariant::One => 1,
            DelayPtrVariant::Two => 2,
            DelayPtrVariant::Three => 3,
            DelayPtrVariant::Six => 6,
            DelayPtrVariant::Twelve => 12,
        }
    }
}
