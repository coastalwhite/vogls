use std::fmt;

use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::{Bits, GlobalContext, LogicMode, SignalKey, Time, VariableKey};

/// A pointer to a certain amount of delay values.
///
/// This is a bit-packed structure that contains an offset (49 bits), triple mask (12 bits) and
/// variant (3 bits).
///
/// - The offset is a element offset into a vector.
/// - The triple mask is a bitmask that determines whether an element stores a `min:typ:max` (when
///   set to 1) or just a single `typ` value (when set to 0).
/// - The variant determines whether there are 1, 2, 3, 6 or 12 elements. The encoding is of the
///   the values of [`DelayPtrVariant`].
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
        debug_assert_eq!(
            triple_mask & ((1u16 << variant.num_delays()) - 1),
            triple_mask
        );

        let offset = offset & 0x1_FFFF_FFFF_FFFF;
        let triple_mask = triple_mask & ((1u16 << variant.num_delays()) - 1);

        Self((offset << 15) | ((triple_mask as u64) << 3) | (variant as u64))
    }

    #[inline(always)]
    pub fn num_triples(self) -> u32 {
        self.triple_mask().count_ones()
    }
    #[inline(always)]
    pub fn num_delays(self) -> usize {
        self.variant().num_delays()
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

    /// A cursor over the delays this pointer names, in declaration order.
    pub fn reader(self, delays: &[Time]) -> DelayReader<'_> {
        DelayReader {
            ptr: self.offset() as usize,
            triple_mask: u64::from(self.triple_mask()),
            delays,
        }
    }

    /// Emit the delay of the `tstart` to `tend` transition of bit `bitidx`.
    pub fn calculate(
        self,
        delays: &[Time],
        gl: &mut GlobalContext,
        builder: &mut vogls_ir::BasicBlockBuilder,
        tstart: SignalKey,
        tend: SignalKey,
        bitidx: u32,
    ) -> VariableKey {
        use DelayPtrVariant as V;

        let tstart_mode = gl.signals[tstart].mode;
        let tend_mode = gl.signals[tend].mode;

        assert_eq!(tstart_mode, tend_mode);

        let variant = self.variant();
        let mut reader = self.reader(delays);

        if variant == V::One {
            // @Performance: At the moment, this is still a variable wait. We could propagate this
            // constant up so that the wait can be become a constant wait. This should allow for
            // better optimization / codegen further on.
            return builder.constant_u64(gl, reader.get().0);
        }

        // Fast path: Two value logic.
        if tstart_mode == LogicMode::TwoValue {
            // Every variant but `One` leads with the rise and fall delays, and two-value logic
            // only ever transitions between 0 and 1. The remaining delays stay unread.
            let t01 = reader.get().0;
            let t10 = reader.get().0;

            let mut out_delay = builder.constant_u64(gl, t01);
            if t01 != t10 {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let is_t10 = builder.ornot(gl, tstart, tend);
                let t10_delay = builder.constant_u64(gl, t10);
                out_delay = builder.select(gl, is_t10, t10_delay, out_delay)
            }
            return out_delay;
        }

        if variant == V::Two {
            // Derived from Chapter 14 of IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001)
            //
            // | f | t | delay             | C |
            // |---|---|-------------------|---|
            // | 0 | 1 | trise             | 0 |
            // | 1 | 0 | tfall             | 1 |
            // | 0 | z | trise             | 0 |
            // | z | 1 | trise             | 0 |
            // | 1 | z | tfall             | 1 |
            // | z | 0 | tfall             | 1 |
            // | 0 | x | trise             | 0 |
            // | x | 1 | trise             | 0 |
            // | 1 | x | tfall             | 1 |
            // | x | 0 | tfall             | 1 |
            // | x | z | max(trise, tfall) | 2 |
            // | z | x | min(trise, tfall) | 3 |
            //
            // C0: (t == 1 || f == 0) -> trise
            // C1: (f == 1 || t == 0) -> tfall
            // C2: ({f,t} == 2'bxz)   -> max(trise, tfall)
            // C3: ({f,t} == 2'bzx)   -> min(trise, tfall)

            let trise = reader.get().0;
            let tfall = reader.get().0;

            let mut out_delay = builder.constant_u64(gl, trise);
            if trise != tfall {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let x = builder.constant(gl, Bits::from(FvLogicValue::X));
                let z = builder.constant(gl, Bits::from(FvLogicValue::Z));
                let zero = builder.constant(gl, Bits::from(false));
                let one = builder.constant(gl, Bits::from(true));

                // C1
                let start_is_one = builder.case_equals(gl, tstart, one);
                let end_is_zero = builder.case_equals(gl, tend, zero);
                let is_c1 = builder.or(gl, start_is_one, end_is_zero);
                let c1_delay = builder.constant_u64(gl, tfall);
                out_delay = builder.select(gl, is_c1, c1_delay, out_delay);

                // C2
                let start_is_x = builder.case_equals(gl, tstart, x);
                let end_is_z = builder.case_equals(gl, tend, z);
                let is_c2 = builder.and(gl, start_is_x, end_is_z);
                let c2_delay = builder.constant_u64(gl, u64::max(trise, tfall));
                out_delay = builder.select(gl, is_c2, c2_delay, out_delay);

                // C3
                let start_is_z = builder.case_equals(gl, tstart, z);
                let end_is_x = builder.case_equals(gl, tend, x);
                let is_c3 = builder.and(gl, start_is_z, end_is_x);
                let c3_delay = builder.constant_u64(gl, u64::min(trise, tfall));
                out_delay = builder.select(gl, is_c3, c3_delay, out_delay);
            }
            return out_delay;
        }

        if variant == V::Three {
            // Derived from Chapter 14 of IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001)
            //
            // | f | t | delay             | C |
            // |---|---|-------------------|---|
            // | 0 | 1 | trise             | 0 |
            // | 1 | 0 | tfall             | 1 |
            // | 0 | z | tz                | 2 |
            // | z | 1 | trise             | 0 |
            // | 1 | z | tz                | 2 |
            // | z | 0 | tfall             | 1 |
            // | 0 | x | min(tz, trise)    | 3 |
            // | x | 1 | trise             | 0 |
            // | 1 | x | min(tz, tfall)    | 4 |
            // | x | 0 | tfall             | 1 |
            // | x | z | tz                | 2 |
            // | z | x | min(tfall, trise) | 5 |
            //
            // C0: (t == 1 || f == 0) -> trise
            // C1: (f == 1 || t == 0) -> tfall
            // C2: (t == 1'bz)        -> tz
            // C3: ({f,t} == 2'b0x)   -> min(tz, trise)
            // C4: ({f,t} == 2'b1x)   -> min(tz, tfall)
            // C5: ({f,t} == 2'bzx)   -> min(tfall, trise)

            let trise = reader.get().0;
            let tfall = reader.get().0;
            let tz = reader.get().0;

            let mut out_delay = builder.constant_u64(gl, trise);
            if trise != tfall || trise != tz {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let zero = builder.constant(gl, Bits::from(false));
                let one = builder.constant(gl, Bits::from(true));
                let x = builder.constant(gl, Bits::from(FvLogicValue::X));
                let z = builder.constant(gl, Bits::from(FvLogicValue::Z));

                // C1
                if trise != tfall {
                    let start_is_one = builder.case_equals(gl, tstart, one);
                    let end_is_zero = builder.case_equals(gl, tend, zero);
                    let is_c1 = builder.or(gl, start_is_one, end_is_zero);
                    let c1_delay = builder.constant_u64(gl, tfall);
                    out_delay = builder.select(gl, is_c1, c1_delay, out_delay);
                }

                // C2
                if trise != tz {
                    let is_c2 = builder.case_equals(gl, tend, z);
                    let c2_delay = builder.constant_u64(gl, tz);
                    out_delay = builder.select(gl, is_c2, c2_delay, out_delay);
                }

                // C3
                if trise != tz {
                    let start_is_zero = builder.case_equals(gl, tstart, zero);
                    let end_is_x = builder.case_equals(gl, tend, x);
                    let is_c3 = builder.and(gl, start_is_zero, end_is_x);
                    let c3_delay = builder.constant_u64(gl, u64::min(tz, trise));
                    out_delay = builder.select(gl, is_c3, c3_delay, out_delay);
                }

                // C4
                if tfall != tz {
                    let start_is_one = builder.case_equals(gl, tstart, one);
                    let end_is_x = builder.case_equals(gl, tend, x);
                    let is_c4 = builder.and(gl, start_is_one, end_is_x);
                    let c4_delay = builder.constant_u64(gl, u64::min(tz, tfall));
                    out_delay = builder.select(gl, is_c4, c4_delay, out_delay);
                }

                // C5
                if trise != tfall {
                    let start_is_z = builder.case_equals(gl, tstart, z);
                    let end_is_x = builder.case_equals(gl, tend, x);

                    let is_c5 = builder.and(gl, start_is_z, end_is_x);
                    let c5_delay = builder.constant_u64(gl, u64::min(trise, tfall));
                    out_delay = builder.select(gl, is_c5, c5_delay, out_delay);
                }
            }
            return out_delay;
        }

        let t01 = reader.get().0;
        let t10 = reader.get().0;
        let t0z = reader.get().0;
        let tz1 = reader.get().0;
        let t1z = reader.get().0;
        let tz0 = reader.get().0;
        let (t0x, tx1, t1x, tx0, txz, tzx) = match variant {
            // The six `x` transitions are derived from the ones that were given.
            V::Six => (
                t0z.min(t01),
                tz1.max(t01),
                t1z.min(t10),
                tz0.max(t10),
                t1z.max(t0z),
                tz1.min(tz0),
            ),
            V::Twelve => (
                reader.get().0,
                reader.get().0,
                reader.get().0,
                reader.get().0,
                reader.get().0,
                reader.get().0,
            ),
            V::One | V::Two | V::Three => unreachable!("handled above"),
        };

        // @Performance
        // We should be able to do some better delay merging here.
        let mut out_delay = builder.constant_u64(gl, t01);
        let tstart = builder.probe(gl, tstart);
        let tstart = builder.select_bit_constant(gl, tstart, bitidx);
        let tend = builder.probe(gl, tend);
        let tend = builder.select_bit_constant(gl, tend, bitidx);
        let zero = builder.constant(gl, Bits::from(false));
        let one = builder.constant(gl, Bits::from(true));
        let x = builder.constant(gl, Bits::from(FvLogicValue::X));
        let z = builder.constant(gl, Bits::from(FvLogicValue::Z));
        macro_rules! transition {
            ($t:expr, $l:expr, $r:expr) => {
                if t01 != $t {
                    let is_start = builder.case_equals(gl, tstart, $l);
                    let is_end = builder.case_equals(gl, tend, $r);
                    let is_transition = builder.and(gl, is_start, is_end);
                    let transition_time = builder.constant_u64(gl, $t);
                    out_delay = builder.select(gl, is_transition, transition_time, out_delay);
                }
            };
        }
        transition!(t10, one, zero);
        transition!(t0z, zero, z);
        transition!(tz1, z, one);
        transition!(t1z, one, z);
        transition!(tz0, z, zero);
        transition!(t0x, zero, x);
        transition!(tx1, x, one);
        transition!(t1x, one, x);
        transition!(tx0, x, zero);
        transition!(txz, x, z);
        transition!(tzx, z, x);

        out_delay
    }
}

/// A cursor over the delays a [`DelayPtr`] names.
///
/// Path delay lists are positional, so [`DelayReader::get`] has to be called once per delay of the
/// declaration, in order. Delays past the ones a caller needs stay unread.
pub struct DelayReader<'a> {
    ptr: usize,
    triple_mask: u64,
    delays: &'a [Time],
}

impl DelayReader<'_> {
    /// The next delay of the declaration.
    pub fn get(&mut self) -> Time {
        let is_triple = self.triple_mask & 1 != 0;
        self.triple_mask >>= 1;

        // A triple is stored as `min`, `typ`, `max`.
        //
        // @TODO: Make the choice between the three configurable.
        let time = self.delays[self.ptr + usize::from(is_triple)];
        self.ptr += if is_triple { 3 } else { 1 };

        time
    }
}

impl DelayPtrVariant {
    /// How many delay values this variant names.
    pub fn num_delays(self) -> usize {
        match self {
            DelayPtrVariant::One => 1,
            DelayPtrVariant::Two => 2,
            DelayPtrVariant::Three => 3,
            DelayPtrVariant::Six => 6,
            DelayPtrVariant::Twelve => 12,
        }
    }
}
