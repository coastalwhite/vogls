use hashbrown::hash_map::Entry;
use vogls_utils::{IndexSet, VgHashMap, VgHashSet};

use crate::form::check_ir_form;
use crate::token_range::TokenRange;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification,
    BinaryOp, Bits, GlobalContext, INTEGER_VSIZE, Instruction, IntrinsicOp, Process, ProcessKey,
    ProcessKind, ResizeOp, SCALAR_VSIZE, SignalFlags, SignalKey, TIME_VSIZE, TemporalRegionKey,
    Time, UnaryOp, VSIZE_32, VSIZE_64, VariableKey, VectorSize,
};

#[must_use]
pub struct BasicBlockBuilder {
    key: BasicBlockKey,
    pub instrs: Vec<Instruction>,
}

pub struct ProcessBuilder {
    key: Option<ProcessKey>,
    entry: TemporalRegionKey,
}

impl ProcessBuilder {
    pub fn new_anonymous(gl: &'_ mut GlobalContext) -> (Self, BasicBlockBuilder) {
        let bb_key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let region = TemporalRegionKey::from_entry(bb_key);
        gl.bbs[bb_key].region = region;
        (
            Self {
                key: None,
                entry: region,
            },
            BasicBlockBuilder {
                key: bb_key,
                instrs: Vec::new(),
            },
        )
    }

    pub fn new(
        gl: &'_ mut GlobalContext,
        kind: ProcessKind,
        origin: TokenRange,
    ) -> (Self, BasicBlockBuilder) {
        let bb_key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let region = TemporalRegionKey::from_entry(bb_key);
        gl.bbs[bb_key].region = region;
        let process_key = gl.processes.insert(Process {
            kind,
            regions: vec![region],
            origin,
        });
        (
            Self {
                key: Some(process_key),
                entry: region,
            },
            BasicBlockBuilder {
                key: bb_key,
                instrs: Vec::new(),
            },
        )
    }

    pub fn key(&self) -> Option<ProcessKey> {
        self.key
    }

    pub fn finalize(self, gl: &mut GlobalContext) {
        let mut stack = Vec::<BasicBlockKey>::new();
        let mut seen = VgHashSet::<BasicBlockKey>::default();
        let mut temporals = VgHashMap::<BasicBlockKey, TemporalRegionKey>::default();
        let mut temporal_roots = IndexSet::<TemporalRegionKey>::default();
        let mut var_def_region = VgHashMap::<VariableKey, TemporalRegionKey>::default();

        temporal_roots.insert(self.entry);
        seen.insert(self.entry.entry());
        stack.push(self.entry.entry());

        // Find all initial temporal roots. These are the basic blocks that are a temporal terminator
        // points to.
        while let Some(bb_key) = stack.pop() {
            let bb = &gl.bbs[bb_key];
            bb.terminator.for_each_temporal_bb(|bb| {
                if seen.insert(bb) {
                    stack.push(bb);
                }
            });

            if bb.terminator.is_temporal() {
                bb.terminator.for_each_temporal_bb(|bb| {
                    temporal_roots.insert(TemporalRegionKey::from_entry(bb));
                });
            }
        }

        // Iterate all temporal roots and perform a graph traversal from them.
        // - If they find a BB that has not been assigned a temporal region, mark it as the current
        // region.
        // - If they find a BB that has been assigned a temporal region, mark it as a new root if it
        // was not already.
        let mut temporal_region = 0;
        while temporal_region < temporal_roots.len() {
            let &root = temporal_roots.get_at_index(temporal_region).unwrap();

            seen.clear();
            stack.push(root.entry());
            seen.insert(root.entry());
            temporals.insert(root.entry(), root);

            while let Some(bb_key) = stack.pop() {
                let bb = &gl.bbs[bb_key];

                // Only traverse through non-temporal edges.
                bb.terminator.for_each_non_temporal_bb(|bb| {
                    if seen.insert(bb) {
                        match temporals.entry(bb) {
                            Entry::Vacant(entry) => {
                                entry.insert(root);
                                stack.push(bb);
                            }
                            Entry::Occupied(mut entry) => {
                                entry.insert(TemporalRegionKey::from_entry(bb));
                                temporal_roots.insert(TemporalRegionKey::from_entry(bb));
                            }
                        }
                    }
                });
            }
            temporal_region += 1;
        }

        for (&bb_key, &region) in &temporals {
            let bb = &mut gl.bbs[bb_key];
            bb.region = region;

            for i in &bb.instrs {
                if let Some(dst) = i.get_destination_variable() {
                    var_def_region.insert(dst, region);
                }
            }

            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Wait(..) | T::VariableWait(..) | T::WaitRegion(..) | T::Watch(..) | T::Halt => {}

                T::Jump(tgt) => {
                    let tgt_tr = temporals[tgt];
                    if tgt_tr != region {
                        debug_assert_eq!(tgt_tr, TemporalRegionKey::from_entry(*tgt));
                        bb.terminator = T::Wait(tgt_tr, Time(0));
                    }
                }
                T::Branch(condition, truthy, falsy) => {
                    let (condition, mut truthy, mut falsy) = (*condition, *truthy, *falsy);
                    let truthy_tr = temporals[&truthy];
                    let falsy_tr = temporals[&falsy];

                    if region == truthy_tr && region == falsy_tr {
                        continue;
                    }

                    if truthy_tr != region {
                        debug_assert_eq!(truthy_tr, TemporalRegionKey::from_entry(truthy));
                        truthy = gl.bbs.insert(BasicBlock {
                            instrs: Vec::new(),
                            region,
                            terminator: BasicBlockTerminator::Wait(truthy_tr, Time(0)),
                        });
                    }
                    if falsy_tr != region {
                        debug_assert_eq!(falsy_tr, TemporalRegionKey::from_entry(falsy));
                        falsy = gl.bbs.insert(BasicBlock {
                            instrs: Vec::new(),
                            region,
                            terminator: BasicBlockTerminator::Wait(falsy_tr, Time(0)),
                        });
                    }
                    gl.bbs[bb_key].terminator = T::Branch(condition, truthy, falsy);
                }
            }
        }

        // Turn temporal variables (i.e. variables that span several temporal regions) into
        // signals.
        //
        // We insert probes at the start of each basic block that uses them.
        // We insert drives at the end of each basic block that uses them.
        let mut temporal_var_to_signal = VgHashMap::<VariableKey, SignalKey>::default();
        let mut temporal_vars = IndexSet::<VariableKey>::default();
        let mut var_remap = VgHashMap::<VariableKey, VariableKey>::default();
        for (&bb_key, &region) in &temporals {
            temporal_vars.clear();
            var_remap.clear();

            let bb = &mut gl.bbs[bb_key];
            bb.region = region;

            for i in &mut bb.instrs {
                i.for_each_src(|src| {
                    if var_def_region[&src] != region {
                        if matches!(i, Instruction::Phi(..)) {
                            panic!("Temporal variables are not allowed in Phi instructions");
                        }

                        temporal_vars.insert(src);
                    }
                });
            }

            if temporal_vars.is_empty() {
                continue;
            }

            let mut instrs = Vec::with_capacity(bb.instrs.len() + temporal_vars.len());

            for &v in temporal_vars.iter() {
                let size = gl.vars.size(v);
                let signal = match temporal_var_to_signal.entry(v) {
                    Entry::Vacant(entry) => {
                        let signal = gl.signals.insert(crate::Signal {
                            name: format!("TEMPORAL_VAR/{}", gl.signals.len()),
                            size,
                            initialize: None,
                            flags: SignalFlags::EMPTY,
                            origin: TokenRange::default(),
                        });
                        entry.insert(signal);
                        signal
                    }
                    Entry::Occupied(entry) => *entry.get(),
                };

                let dst = gl.vars.insert(size);
                instrs.push(Instruction::Probe(dst, signal, 0));
                var_remap.insert(v, dst);
            }

            instrs.extend(bb.instrs.drain(..).map(|mut i| {
                i.map_src_vars(|v| var_remap.get(&v).copied().unwrap_or(v));
                i
            }));
            bb.instrs = instrs;
        }

        if !temporal_var_to_signal.is_empty() {
            for (&bb_key, _) in &temporals {
                temporal_vars.clear();
                let bb = &mut gl.bbs[bb_key];

                for instr in &bb.instrs {
                    let Some(var) = instr
                        .get_destination_variable()
                        .filter(|var| temporal_var_to_signal.contains_key(var))
                    else {
                        continue;
                    };
                    temporal_vars.insert(var);
                }

                if temporal_vars.is_empty() {
                    continue;
                }
                bb.instrs.extend(temporal_vars.iter().map(|var| {
                    let signal = temporal_var_to_signal[var];
                    Instruction::Drive(signal, *var, None)
                }));
            }
        }

        let regions = temporal_roots.take_keys();
        check_ir_form(&regions, gl);

        if let Some(key) = self.key {
            gl.processes[key].regions = regions;
        }
    }
}

pub struct PhiRef(BasicBlockKey, usize);
pub struct BranchRef(BasicBlockKey);

impl BranchRef {
    pub fn origin_key(&self) -> BasicBlockKey {
        self.0
    }
}

macro_rules! arithmetic_op {
    ($(($name:ident, $op:ident),)+) => {
        $(
        pub fn $name(
            &mut self,
            gl: &mut GlobalContext,
            lhs: VariableKey,
            rhs: VariableKey,
        ) -> VariableKey {
            self.bin_arithmetic(gl, lhs, rhs, BinaryOp::$op)
        }
        )+
    }
}

impl BasicBlockBuilder {
    pub fn key(&self) -> BasicBlockKey {
        self.key
    }

    pub fn instrs_len(&self) -> usize {
        self.instrs.len()
    }

    pub fn next_bb_non_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        })
    }

    pub fn next_bb_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockKey {
        let key = gl.bbs.insert(BasicBlock {
            instrs: Vec::new(),
            region: TemporalRegionKey::default(),
            terminator: BasicBlockTerminator::Halt,
        });
        let region = TemporalRegionKey::from_entry(key);
        gl.bbs[key].region = region;
        key
    }

    pub fn next_builder_non_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb_non_temporal(gl);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }
    pub fn next_builder_temporal(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_key = self.next_bb_non_temporal(gl);
        BasicBlockBuilder {
            key: next_key,
            instrs: Vec::new(),
        }
    }

    pub fn phi(
        &mut self,
        gl: &mut GlobalContext,
        srcs: Box<[(BasicBlockKey, VariableKey)]>,
    ) -> (VariableKey, PhiRef) {
        assert!(!srcs.is_empty());
        let (_, var) = srcs.first().unwrap();
        let size = gl.vars.size(*var);
        assert!(srcs.iter().all(|(_, v)| size == gl.vars.size(*v)));
        let dst = gl.vars.insert(size);
        let offset = self.instrs.len();
        self.instrs.push(Instruction::Phi(dst, srcs));
        (dst, PhiRef(self.key(), offset))
    }

    pub fn update_phi_ref(
        &mut self,
        gl: &mut GlobalContext,
        phi_ref: PhiRef,
        idx: usize,
        bb: BasicBlockKey,
        var: VariableKey,
    ) {
        let instr = if self.key() == phi_ref.0 {
            &mut self.instrs[phi_ref.1]
        } else {
            &mut gl.bbs[phi_ref.0].instrs[phi_ref.1]
        };
        let Instruction::Phi(_, srcs) = instr else {
            panic!("not a phi");
        };
        srcs[idx] = (bb, var);
    }

    pub fn update_branch_truthy(
        &mut self,
        gl: &mut GlobalContext,
        branch_ref: BranchRef,
        bb: BasicBlockKey,
    ) {
        let BasicBlockTerminator::Branch(_, truthy, _) = &mut gl.bbs[branch_ref.0].terminator
        else {
            panic!("not a branch");
        };
        *truthy = bb;
    }

    pub fn update_branch_falsy(
        &mut self,
        gl: &mut GlobalContext,
        branch_ref: BranchRef,
        bb: BasicBlockKey,
    ) {
        let BasicBlockTerminator::Branch(_, _, falsy) = &mut gl.bbs[branch_ref.0].terminator else {
            panic!("not a branch");
        };
        *falsy = bb;
    }

    pub fn constant(&mut self, gl: &mut GlobalContext, value: Bits) -> VariableKey {
        let variable = gl.vars.insert(value.size());
        self.instrs.push(Instruction::Constant(variable, value));
        variable
    }

    pub fn constant_u32(&mut self, gl: &mut GlobalContext, value: u32) -> VariableKey {
        let variable = gl.vars.insert(VSIZE_32);
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u32(value)));
        variable
    }
    pub fn constant_u64(&mut self, gl: &mut GlobalContext, value: u64) -> VariableKey {
        let variable = gl.vars.insert(VSIZE_64);
        self.instrs
            .push(Instruction::Constant(variable, Bits::new_u64(value)));
        variable
    }

    pub fn concat(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs_size = gl.vars.size(lhs);
        let rhs_size = gl.vars.size(rhs);
        let size = VectorSize::new(lhs_size.get() + rhs_size.get()).unwrap();
        let dst = gl.vars.insert(size);
        self.bin_op(gl, lhs, rhs, BinaryOp::Concat, dst);
        dst
    }

    pub fn binary_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars.size(src);
        let dst = gl.vars.insert(size);
        self.unary_op(gl, src, UnaryOp::Neg, dst);
        dst
    }

    pub fn logical_neg(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars.size(src);
        let src = match size.get() {
            1 => src,
            _ => self.reduce_or(gl, src),
        };
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::Neg, dst);
        dst
    }

    pub fn unary_op(
        &mut self,
        _gl: &mut GlobalContext,
        src: VariableKey,
        op: UnaryOp,
        dst: VariableKey,
    ) {
        self.instrs.push(Instruction::Unary(dst, op, src));
    }
    pub fn resize_op(
        &mut self,
        _gl: &mut GlobalContext,
        dst: VariableKey,
        op: ResizeOp,
        src: VariableKey,
    ) {
        self.instrs.push(Instruction::Resize(dst, op, src));
    }
    pub fn bin_op(
        &mut self,
        _gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
        dst: VariableKey,
    ) {
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
    }
    pub fn bin_imm_op(
        &mut self,
        src: VariableKey,
        imm: Bits,
        op: BinaryImmOp,
        dst: &mut VariableKey,
    ) {
        match op.simplify(*dst, src, &imm) {
            BinaryImmOpSimplification::Keep => {
                self.instrs.push(Instruction::BinaryImm(*dst, op, src, imm))
            }
            BinaryImmOpSimplification::Source => *dst = src,
            BinaryImmOpSimplification::Immediate => {
                self.instrs.push(Instruction::Constant(*dst, imm))
            }
            BinaryImmOpSimplification::Constant(bits) => {
                self.instrs.push(Instruction::Constant(*dst, bits))
            }
            BinaryImmOpSimplification::Instruction(i) => self.instrs.push(i),
        }
    }
    fn copy_op(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let lhs_size = gl.vars.size(lhs);
        let rhs_size = gl.vars.size(rhs);
        assert_eq!(lhs_size, rhs_size);
        let dst = gl.vars.insert(lhs_size);
        self.bin_op(gl, lhs, rhs, op, dst);
        dst
    }

    pub fn bin_arithmetic(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let lhs_size = gl.vars.size(lhs);
        let rhs_size = gl.vars.size(rhs);
        assert_eq!(lhs_size, rhs_size);
        let dst = gl.vars.insert(lhs_size);
        self.bin_op(gl, lhs, rhs, op, dst);
        dst
    }
    pub fn bin_imm_arithmetic(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
        op: BinaryImmOp,
    ) -> VariableKey {
        let size = gl.vars.size(src);
        assert_eq!(size, imm.size());
        let mut dst = gl.vars.insert(size);
        self.bin_imm_op(src, imm, op, &mut dst);
        dst
    }

    // Bitwise Operations
    arithmetic_op! {
        (and, And),
        (or, Or),
        (xor, Xor),
        (plus, Add),
        (minus, Sub),
        (multiply, Multiply),
        (divide, Divide),
        (modulus, Modulus),
        (power, Power),
    }
    pub fn xnor(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let xnor = self.binary_neg(gl, xor);
        xnor
    }
    pub fn andnot(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let rhs = self.binary_neg(gl, rhs);
        self.and(gl, lhs, rhs)
    }
    pub fn ornot(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let rhs = self.binary_neg(gl, rhs);
        self.or(gl, lhs, rhs)
    }

    // Bitwise Immediate Operations
    pub fn and_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return src;
        } else if num_special == 0 && num_ones == 0 {
            return self.constant(gl, imm);
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::And)
    }
    pub fn or_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return self.constant(gl, imm);
        } else if num_special == 0 && num_ones == 0 {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Or)
    }
    pub fn xor_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        let num_special = imm.count_special();
        if num_special == imm.size().get() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        let num_ones = imm.count_ones();
        if num_ones == imm.size().get() {
            return self.binary_neg(gl, src);
        } else if num_special == 0 && num_ones == 0 {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Xor)
    }
    pub fn plus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Add)
    }
    pub fn minus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Sub)
    }
    pub fn multiply_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_zeroed(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Multiply)
    }
    pub fn power_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_u32(1).truncate_or_zero_extend(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn divide_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_ones(imm.size()));
        } else if imm.eq_one() {
            return src;
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn modulus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_zero() {
            return self.constant(gl, Bits::new_ones(imm.size()));
        } else if imm.eq_one() {
            return self.constant(gl, Bits::new_zeroed(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::Power)
    }
    pub fn revminus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevSub)
    }
    pub fn revpower_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        if imm.eq_one() {
            return self.constant(gl, imm);
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevPower)
    }
    pub fn revdivide_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevDivide)
    }
    pub fn revmodulus_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(src), imm.size());
        if imm.contains_special() {
            return self.constant(gl, Bits::new_unknown(imm.size()));
        }

        self.bin_imm_arithmetic(gl, src, imm, BinaryImmOp::RevModulus)
    }

    pub fn copy_x(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.copy_op(gl, lhs, rhs, BinaryOp::CopyX)
    }
    pub fn copy_z(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.copy_op(gl, lhs, rhs, BinaryOp::CopyZ)
    }

    pub fn count_leading_zeros(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let dst = gl.vars.insert(VSIZE_32);
        self.unary_op(gl, src, UnaryOp::LeadingZeros, dst);
        dst
    }

    pub fn select_bit(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: VariableKey,
    ) -> VariableKey {
        self.slice(gl, src, idx, SCALAR_VSIZE)
    }
    pub fn select_bit_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        idx: u32,
    ) -> VariableKey {
        self.slice_constant(gl, src, idx, SCALAR_VSIZE)
    }
    pub fn truncate(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let size = gl.vars.size(src);
        if size == width {
            return src;
        }

        let dst = gl.vars.insert(width);
        self.resize_op(gl, dst, ResizeOp::Truncate, src);
        dst
    }
    pub fn slice_constant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let src_size = gl.vars.size(src);
        assert!(gl.vars.size(src) >= width);
        if offset == 0 {
            return self.truncate(gl, src, width);
        }
        if offset >= src_size.get() {
            return self.constant(gl, Bits::new_unknown(width));
        }
        let dst = gl.vars.insert(width);
        self.instrs.push(Instruction::SliceImm(dst, src, offset));
        if offset <= src_size.get() - width.get() {
            return dst;
        }

        // Slice and SliceImm are slightly different in that out-of-bounds values are set as
        // unknown for Slice and set as zeros for SliceImm. We compensate to compensate for that
        // here.
        self.or_constant(
            gl,
            dst,
            Bits::new_unknown(width).logical_shift_left(src_size.get() - offset),
        )
    }
    pub fn slice(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(offset), INTEGER_VSIZE);
        assert!(gl.vars.size(src) >= width);
        let dst = gl.vars.insert(width);
        self.instrs.push(Instruction::Slice(dst, src, offset));
        dst
    }

    pub fn unsigned_lt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let ge = self.unsigned_le(gl, rhs, lhs);
        self.logical_neg(gl, ge)
    }
    pub fn unsigned_gt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let le = self.unsigned_le(gl, lhs, rhs);
        self.logical_neg(gl, le)
    }
    pub fn unsigned_le(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(lhs), gl.vars.size(rhs));
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.bin_op(gl, lhs, rhs, BinaryOp::UnsignedLessEqual, dst);
        dst
    }
    pub fn unsigned_ge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.unsigned_le(gl, rhs, lhs)
    }

    pub fn signed_lt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let ge = self.signed_le(gl, rhs, lhs);
        self.logical_neg(gl, ge)
    }
    pub fn signed_gt(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let le = self.signed_le(gl, lhs, rhs);
        self.logical_neg(gl, le)
    }
    pub fn signed_le(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(lhs), gl.vars.size(rhs));
        let mask = Bits::new_with_msb_one(gl.vars.size(lhs));
        let lhs = self.xor_constant(gl, lhs, mask.clone());
        let rhs = self.xor_constant(gl, rhs, mask);
        self.unsigned_le(gl, lhs, rhs)
    }
    pub fn signed_ge(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.signed_le(gl, rhs, lhs)
    }

    pub fn logical_or(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.reduce_or(gl, lhs);
        let rhs = self.reduce_or(gl, rhs);
        self.or(gl, lhs, rhs)
    }
    pub fn logical_and(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let lhs = self.reduce_or(gl, lhs);
        let rhs = self.reduce_or(gl, rhs);
        self.and(gl, lhs, rhs)
    }

    pub fn equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let no_equals = self.reduce_or(gl, xor);
        let xnor = self.binary_neg(gl, no_equals);
        xnor
    }
    pub fn not_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let xor = self.xor(gl, lhs, rhs);
        let no_equals = self.reduce_or(gl, xor);
        no_equals
    }
    pub fn case_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(lhs), gl.vars.size(rhs));
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.bin_op(gl, lhs, rhs, BinaryOp::CaseEquality, dst);
        dst
    }
    pub fn not_case_equals(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        let case_equals = self.case_equals(gl, lhs, rhs);
        self.logical_neg(gl, case_equals)
    }

    pub fn case_equals_constant(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        imm: Bits,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(lhs), imm.size());
        let mut dst = gl.vars.insert(SCALAR_VSIZE);
        self.bin_imm_op(lhs, imm, BinaryImmOp::CaseEquality, &mut dst);
        dst
    }

    pub fn reduce_xor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars.size(src);
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceXor, dst);
        dst
    }
    pub fn reduce_or(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars.size(src);
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceOr, dst);
        dst
    }
    pub fn reduce_and(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let size = gl.vars.size(src);
        if size == SCALAR_VSIZE {
            return src;
        }

        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.unary_op(gl, src, UnaryOp::ReduceAnd, dst);
        dst
    }
    pub fn reduce_xnor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let xor = self.reduce_xor(gl, src);
        self.logical_neg(gl, xor)
    }
    pub fn reduce_nor(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let or = self.reduce_or(gl, src);
        self.logical_neg(gl, or)
    }
    pub fn reduce_nand(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let and = self.reduce_and(gl, src);
        self.logical_neg(gl, and)
    }

    pub fn drive(&mut self, gl: &mut GlobalContext, signal: SignalKey, src: VariableKey) {
        self.drive_opt_partial(gl, signal, src, None);
    }
    pub fn drive_partial_constant(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: u32,
    ) {
        if offset == 0 {
            return self.drive_opt_partial(gl, signal, src, None);
        }

        let offset = self.constant_u32(gl, offset);
        self.drive_opt_partial(gl, signal, src, Some((offset, gl.signals[signal].size)));
    }
    pub fn drive_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        offset: VariableKey,
    ) {
        self.drive_opt_partial(gl, signal, src, Some((offset, gl.signals[signal].size)));
    }
    pub fn drive_opt_partial(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        src: VariableKey,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        if let Some((offset, _)) = partial {
            assert_eq!(gl.vars.size(offset), INTEGER_VSIZE);
        }
        self.instrs.push(Instruction::Drive(signal, src, partial));
    }

    pub fn probe(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let size = gl.signals.get(signal).unwrap().size;
        let dst = gl.vars.insert(size);
        self.instrs.push(Instruction::Probe(dst, signal, 0));
        dst
    }

    pub fn probe_slice(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let dst = gl.vars.insert(width);
        self.instrs
            .push(Instruction::ProbeSlice(dst, signal, offset));
        dst
    }

    pub fn probe_slice_constant(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        offset: u32,
        width: VectorSize,
    ) -> VariableKey {
        let src_size = gl.signals[signal].size;
        let dst = gl.vars.insert(width);
        self.instrs.push(Instruction::Probe(dst, signal, offset));
        if offset <= src_size.get() - width.get() {
            return dst;
        }

        // Slice and SliceImm are slightly different in that out-of-bounds values are set as
        // unknown for Slice and set as zeros for SliceImm. We compensate to compensate for that
        // here.
        self.or_constant(
            gl,
            dst,
            Bits::new_unknown(width).logical_shift_left(src_size.get() - offset),
        )
    }

    fn finalize(&mut self, gl: &mut GlobalContext, terminator: BasicBlockTerminator) {
        let bb = &mut gl.bbs[self.key];
        bb.instrs = std::mem::take(&mut self.instrs);
        bb.terminator = terminator;
    }

    pub fn jump(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        let next_builder = self.next_builder_non_temporal(gl);
        self.finalize(gl, BasicBlockTerminator::Jump(next_builder.key()));
        next_builder
    }
    pub fn jump_to(mut self, gl: &mut GlobalContext, bb: BasicBlockKey) {
        self.finalize(gl, BasicBlockTerminator::Jump(bb));
    }

    pub fn next_terminate_later(&mut self, gl: &mut GlobalContext) -> BasicBlockBuilder {
        self.finalize(gl, BasicBlockTerminator::Halt);
        self.next_builder_non_temporal(gl)
    }

    pub fn continue_from(instrs: Vec<Instruction>, bb: BasicBlockKey) -> BasicBlockBuilder {
        BasicBlockBuilder {
            key: bb,
            instrs: instrs,
        }
    }

    pub fn push_raw_instruction(&mut self, instruction: Instruction) {
        self.instrs.push(instruction);
    }

    pub fn into_instructions(self) -> Vec<Instruction> {
        self.instrs
    }

    pub fn branch(
        &mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
    ) -> (BranchRef, BasicBlockBuilder) {
        assert_eq!(gl.vars.size(condition), SCALAR_VSIZE);
        let branch_bb = self.key();

        let next_builder = self.next_builder_non_temporal(gl);
        let next_key = next_builder.key();
        self.finalize(
            gl,
            BasicBlockTerminator::Branch(condition, next_key, next_key),
        );
        (BranchRef(branch_bb), next_builder)
    }

    pub fn double_branch(
        &mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
    ) -> (BasicBlockBuilder, BasicBlockBuilder) {
        assert_eq!(gl.vars.size(condition), SCALAR_VSIZE);
        let next_builder_truthy = self.next_builder_non_temporal(gl);
        let next_builder_falsy = self.next_builder_non_temporal(gl);
        self.finalize(
            gl,
            BasicBlockTerminator::Branch(
                condition,
                next_builder_truthy.key(),
                next_builder_falsy.key(),
            ),
        );
        (next_builder_truthy, next_builder_falsy)
    }

    pub fn branch_true_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let (bref, builder) = self.branch(gl, condition);
        self.update_branch_truthy(gl, bref, bb);
        builder
    }
    pub fn branch_false_to(
        mut self,
        gl: &mut GlobalContext,
        condition: VariableKey,
        bb: BasicBlockKey,
    ) -> BasicBlockBuilder {
        let (bref, builder) = self.branch(gl, condition);
        self.update_branch_falsy(gl, bref, bb);
        builder
    }

    pub fn halt(mut self, gl: &mut GlobalContext) {
        self.finalize(gl, BasicBlockTerminator::Halt);
    }

    fn temporal_term(
        mut self,
        gl: &mut GlobalContext,
        f: impl FnOnce(TemporalRegionKey) -> BasicBlockTerminator,
    ) -> BasicBlockBuilder {
        let next_builder = self.next_builder_temporal(gl);
        self.finalize(gl, f(TemporalRegionKey::from_entry(next_builder.key())));
        next_builder
    }
    fn temporal_term_to(
        mut self,
        gl: &mut GlobalContext,
        to: BasicBlockKey,
        f: impl FnOnce(TemporalRegionKey) -> BasicBlockTerminator,
    ) {
        self.finalize(gl, f(TemporalRegionKey::from_entry(to)));
    }

    pub fn wait(self, gl: &mut GlobalContext, time: Time) -> BasicBlockBuilder {
        self.temporal_term(gl, |key| BasicBlockTerminator::Wait(key, time))
    }
    pub fn wait_to(self, gl: &mut GlobalContext, time: Time, bb: BasicBlockKey) {
        self.temporal_term_to(gl, bb, |key| BasicBlockTerminator::Wait(key, time))
    }
    pub fn variable_wait(self, gl: &mut GlobalContext, time: VariableKey) -> BasicBlockBuilder {
        self.temporal_term(gl, |key| BasicBlockTerminator::VariableWait(key, time))
    }
    pub fn variable_wait_to(self, gl: &mut GlobalContext, time: VariableKey, bb: BasicBlockKey) {
        self.temporal_term_to(gl, bb, |key| BasicBlockTerminator::VariableWait(key, time))
    }
    pub fn wait_region(self, gl: &mut GlobalContext, region: u8) -> BasicBlockBuilder {
        self.temporal_term(gl, |key| BasicBlockTerminator::WaitRegion(key, region))
    }
    pub fn wait_region_to(self, gl: &mut GlobalContext, region: u8, bb: BasicBlockKey) {
        self.temporal_term_to(gl, bb, |key| BasicBlockTerminator::WaitRegion(key, region))
    }
    pub fn watch(self, gl: &mut GlobalContext, signals: Vec<SignalKey>) -> BasicBlockBuilder {
        self.temporal_term(gl, |key| BasicBlockTerminator::Watch(key, signals))
    }
    pub fn watch_to(self, gl: &mut GlobalContext, signals: Vec<SignalKey>, bb: BasicBlockKey) {
        self.temporal_term_to(gl, bb, |key| BasicBlockTerminator::Watch(key, signals))
    }

    pub fn intrinsic(
        &mut self,
        gl: &mut GlobalContext,
        op: IntrinsicOp,
        args: Box<[VariableKey]>,
    ) -> VariableKey {
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.instrs
            .push(Instruction::Intrinsic(dst, Box::new(op), args));
        dst
    }

    pub fn time(&mut self, gl: &mut GlobalContext) -> VariableKey {
        let dst = gl.vars.insert(TIME_VSIZE);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Time),
            Default::default(),
        ));
        dst
    }
    pub fn random(&mut self, gl: &mut GlobalContext) -> VariableKey {
        let dst = gl.vars.insert(INTEGER_VSIZE);
        self.instrs.push(Instruction::Intrinsic(
            dst,
            Box::new(IntrinsicOp::Random),
            Default::default(),
        ));
        dst
    }

    pub fn zero_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        new_size: VectorSize,
    ) -> VariableKey {
        let size = gl.vars.size(src);
        if size == new_size {
            return src;
        }

        let dst = gl.vars.insert(new_size);
        self.resize_op(gl, dst, ResizeOp::ZeroExtend, src);
        dst
    }
    pub fn sign_extend(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        new_size: VectorSize,
    ) -> VariableKey {
        let size = gl.vars.size(src);
        if size == new_size {
            return src;
        }

        let dst = gl.vars.insert(new_size);
        self.resize_op(gl, dst, ResizeOp::SignExtend, src);
        dst
    }

    pub fn shift(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
        op: BinaryOp,
    ) -> VariableKey {
        let lhs_size = gl.vars.size(lhs);
        assert_eq!(gl.vars.size(rhs), INTEGER_VSIZE);
        let dst = gl.vars.insert(lhs_size);
        self.instrs.push(Instruction::Binary(dst, op, lhs, rhs));
        dst
    }
    pub fn logical_shift_left(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::LogicalShiftLeft)
    }
    pub fn logical_shift_right(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::LogicalShiftRight)
    }
    pub fn arithmetic_shift_right(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.shift(gl, lhs, rhs, BinaryOp::ArithmeticShiftRight)
    }

    pub fn equals_zero(&mut self, gl: &mut GlobalContext, src: VariableKey) -> VariableKey {
        let not_equals_zero = self.reduce_or(gl, src);
        self.logical_neg(gl, not_equals_zero)
    }

    pub fn lupdt(&mut self, gl: &mut GlobalContext, signal: SignalKey) -> VariableKey {
        let dst = gl.vars.insert(TIME_VSIZE);
        self.instrs.push(Instruction::LastUpdateTime(dst, signal));
        dst
    }

    pub fn min(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Min)
    }
    pub fn max(
        &mut self,
        gl: &mut GlobalContext,
        lhs: VariableKey,
        rhs: VariableKey,
    ) -> VariableKey {
        self.bin_arithmetic(gl, lhs, rhs, BinaryOp::Max)
    }
    pub fn select(
        &mut self,
        gl: &mut GlobalContext,
        select: VariableKey,
        truthy: VariableKey,
        falsy: VariableKey,
    ) -> VariableKey {
        let size = gl.vars.size(truthy);
        assert_eq!(size, gl.vars.size(falsy));
        assert_eq!(SCALAR_VSIZE, gl.vars.size(select));
        let dst = gl.vars.insert(size);
        self.instrs
            .push(Instruction::Select(dst, select, truthy, falsy));
        dst
    }

    pub fn posedge(
        &mut self,
        gl: &mut GlobalContext,
        before: VariableKey,
        after: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(before), SCALAR_VSIZE);
        assert_eq!(gl.vars.size(after), SCALAR_VSIZE);
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.bin_op(gl, before, after, BinaryOp::Posedge, dst);
        dst
    }
    pub fn negedge(
        &mut self,
        gl: &mut GlobalContext,
        before: VariableKey,
        after: VariableKey,
    ) -> VariableKey {
        assert_eq!(gl.vars.size(before), SCALAR_VSIZE);
        assert_eq!(gl.vars.size(after), SCALAR_VSIZE);
        let dst = gl.vars.insert(SCALAR_VSIZE);
        self.bin_op(gl, before, after, BinaryOp::Negedge, dst);
        dst
    }

    pub fn minus_revconstant(
        &mut self,
        gl: &mut GlobalContext,
        src: VariableKey,
        constant: Bits,
    ) -> VariableKey {
        let constant = self.constant(gl, constant);
        self.minus(gl, constant, src)
    }

    pub fn rev_imm_slice_x(
        &mut self,
        gl: &mut GlobalContext,
        value: Bits,
        offset: VariableKey,
        width: VectorSize,
    ) -> VariableKey {
        let value = self.constant(gl, value);
        self.slice(gl, value, offset, width)
    }
}
