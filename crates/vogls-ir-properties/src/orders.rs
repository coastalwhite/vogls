use std::ops::ControlFlow;

use slotmap::SlotMap;
use vogls_ir::{BasicBlock, BasicBlockKey};
use vogls_utils::VgHashSet;

fn post_order_impl<E>(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<(bool, BasicBlockKey)>,

    mut f: impl FnMut(BasicBlockKey) -> ControlFlow<E>,
    mut filter: impl FnMut(BasicBlockKey) -> bool,
) -> ControlFlow<E> {
    scratch_seen.clear();
    scratch_stack.clear();

    scratch_stack.push((false, entry));
    while let Some((dispatched, key)) = scratch_stack.pop() {
        if dispatched {
            f(key)?;
            continue;
        }

        scratch_stack.push((true, key));
        bbs[key].terminator.for_each_non_temporal_bb(|bb_key| {
            if filter(bb_key) && scratch_seen.insert(bb_key) {
                scratch_stack.push((false, bb_key));
            }
        });
    }
    ControlFlow::Continue(())
}

pub fn post_order(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<(bool, BasicBlockKey)>,

    mut f: impl FnMut(BasicBlockKey),
    filter: impl FnMut(BasicBlockKey) -> bool,
) {
    _ = post_order_impl(
        entry,
        bbs,
        scratch_seen,
        scratch_stack,
        |bb| {
            f(bb);
            ControlFlow::<()>::Continue(())
        },
        filter,
    );
}

pub fn try_post_order<E>(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<(bool, BasicBlockKey)>,

    mut f: impl FnMut(BasicBlockKey) -> Result<(), E>,
    filter: impl FnMut(BasicBlockKey) -> bool,
) -> Result<(), E> {
    match post_order_impl(
        entry,
        bbs,
        scratch_seen,
        scratch_stack,
        |bb| match f(bb) {
            Ok(()) => ControlFlow::Continue(()),
            Err(err) => ControlFlow::Break(err),
        },
        filter,
    ) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(err) => Err(err),
    }
}

pub fn post_order_keys(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<(bool, BasicBlockKey)>,

    keys: &mut Vec<BasicBlockKey>,
) {
    post_order(
        entry,
        bbs,
        scratch_seen,
        scratch_stack,
        |key| keys.push(key),
        |_| true,
    );
}

fn pre_order_impl<E>(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<BasicBlockKey>,

    mut f: impl FnMut(BasicBlockKey) -> ControlFlow<E>,
    mut filter: impl FnMut(BasicBlockKey) -> bool,
) -> ControlFlow<E> {
    scratch_seen.clear();
    scratch_stack.clear();

    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(key) = scratch_stack.pop() {
        f(key)?;
        bbs[key].terminator.for_each_non_temporal_bb(|bb_key| {
            if filter(bb_key) && scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }
    ControlFlow::Continue(())
}

pub fn pre_order(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<BasicBlockKey>,

    mut f: impl FnMut(BasicBlockKey),
    filter: impl FnMut(BasicBlockKey) -> bool,
) {
    _ = pre_order_impl(entry, bbs, scratch_seen, scratch_stack, |bb| {
        f(bb);
        ControlFlow::<()>::Continue(())
    }, filter);
}

pub fn try_pre_order<E>(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<BasicBlockKey>,

    mut f: impl FnMut(BasicBlockKey) -> Result<(), E>,
    filter: impl FnMut(BasicBlockKey) -> bool,
) -> Result<(), E> {
    match pre_order_impl(entry, bbs, scratch_seen, scratch_stack, |bb| match f(bb) {
        Ok(()) => ControlFlow::Continue(()),
        Err(err) => ControlFlow::Break(err),
    }, filter) {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(err) => Err(err),
    }
}

pub fn pre_order_keys(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_stack: &mut Vec<BasicBlockKey>,

    keys: &mut Vec<BasicBlockKey>,
) {
    pre_order(entry, bbs, scratch_seen, scratch_stack, |key| {
        keys.push(key)
    }, |_| true);
}
