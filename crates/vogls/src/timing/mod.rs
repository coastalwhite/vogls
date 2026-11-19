use std::ops::Range;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::time::{TimeResolution, TimeSize, TimeUnit};
use vogls_ir::token_range::TokenRange;
use vogls_ir::{SignalSlice, Time};
use vogls_sdf::{
    AbsoluteDelayType, DelVal, DelValList, DelayFile, DelaySpec, HierarchicalIdent,
    HierarchyDivider, Instance, Integer, InterconnectDef, IoPathDef, Port, PortInstance, PortSpec,
    RTriple, RValue, SignedRealNumberOrRTriple, Timescale, TimescaleNumber, TimescaleUnit,
    TimingSpec,
};
use vogls_utils::VgHashMap;
use vogls_verilog::elaborate::{SignalSymbol, VSymbol};
use vogls_verilog::lower::delay_pointer::{DelayPtr, DelayPtrVariant};
use vogls_verilog::lower::{Diagnostics, LowerContext, LowerErrorReason, MutLowerContext};

/// Format an SDF hierarchical identifier for diagnostics.
fn display_ident(ident_table: &IdentTable, ident: &HierarchicalIdent) -> String {
    let mut s = ident_table[ident.fst].to_string();
    for (divider, n) in &ident.next {
        s.push(divider.as_char());
        s.push_str(&ident_table[*n]);
    }
    s
}

/// Resolve an SDF hierarchical identifier.
///
/// With a `scope`, the identifier is resolved relative to that scope, which is a cell or the module
/// instance that the SDF file applies to. Without a `scope`, it is an `INSTANCE` path of an SDF file
/// that applies to the top-level module.
fn resolve_ident<'a>(
    ctx: &LowerContext<'a, '_>,
    divider: HierarchyDivider,
    scope: Option<SymbolId>,
    ident: &HierarchicalIdent,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    macro_rules! unknown_ident {
        ($iid:expr) => {
            diagnostics.errors.push((
                TokenRange::default(),
                LowerErrorReason::NotYetImplemented("unknown sdf module"),
                vec![format!("unknown sdf module: {}", &ctx.arenas.ident_table[$iid]).into()],
            ));
            return Err(());
        };
    }

    if let Some((found, _)) = ident.next.iter().find(|(d, _)| *d != divider) {
        diagnostics.errors.push((
            TokenRange::default(),
            LowerErrorReason::SdfDividerMismatch(divider.as_char(), found.as_char()),
            Vec::new(),
        ));
        return Err(());
    }

    let fst = ident.fst;
    let cell_sid = match scope {
        // IEEE Std 1497-2001 (p. 19)
        //
        // """
        // The hierarchical path shall start at the level in the design at which the annotator
        // shall be instructed to apply the SDF file. Frequently, this is the topmost level.
        // """
        None => match ctx.table.roots() {
            &[root] => ctx
                .table
                .resolve(root, fst)
                .or_else(|| ctx.table.resolve_root(fst)),
            _ => ctx.table.resolve_root(fst),
        },
        Some(scope) => ctx.table.resolve(scope, fst),
    };
    let Some(mut cell_sid) = cell_sid else {
        unknown_ident!(fst);
    };
    for &(_, n) in &ident.next {
        let Some(new_cell_sid) = ctx.table.resolve(cell_sid, n) else {
            unknown_ident!(n);
        };
        cell_sid = new_cell_sid;
    }
    Ok(cell_sid)
}

#[allow(clippy::too_many_arguments)]
fn push_sdf_path<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    divider: HierarchyDivider,
    cell_sid: SymbolId,
    (from, to): (&PortInstance, &PortInstance),
    delval_list: &DelValList,
    delay_values: &mut Vec<Time>,
    timescale: TimeResolution,
) -> Result<(), ()> {
    let from_sid = resolve_port_instance(ctx, divider, cell_sid, from, &mut mctx.diagnostics);
    let to_sid = resolve_port_instance(ctx, divider, cell_sid, to, &mut mctx.diagnostics);
    let (from_sid, to_sid) = (from_sid?, to_sid?);

    let (VSymbol::Signal(from_net), VSymbol::Signal(to_net)) =
        (&ctx.table[from_sid].content, &ctx.table[to_sid].content)
    else {
        mctx.diagnostics
            .not_yet_implemented(TokenRange::default(), "Not a net");
        return Err(());
    };

    let from_bits = port_bits(ctx, from_net, from, &mut mctx.diagnostics);
    let to_bits = port_bits(ctx, to_net, to, &mut mctx.diagnostics);
    let (from_bits, to_bits) = (from_bits?, to_bits?);

    let before = delay_values.len();
    let delays = parse_delvallist(delval_list, delay_values, timescale, ctx.time_resolution)?;

    if from_bits.is_some() || to_bits.is_some() {
        let is_zero = delay_values[before..].iter().all(|t| t.0 == 0);
        delay_values.truncate(before);
        // A zero delay adds no delay, also between the bits of a port, e.g. those of the pads of a
        // padring that connect to the bits of the ports of the top-level module.
        if is_zero {
            return Ok(());
        }
        mctx.diagnostics.not_yet_implemented(
            TokenRange::default(),
            "a non-zero SDF delay from or to a bit or part select of a port",
        );
        return Err(());
    }

    let from = from_net.terminals.probe_signal();
    let to = to_net.terminals.probe_signal();
    mctx.driver_resolution
        .path_delays
        .push_sdf(from, to, to_sid, delays);
    Ok(())
}

/// Returns the bits of `net` that the bit select `[b1]` or part select `[b1:b2]` of the SDF `port`
/// selects, or `None` if it selects the whole net.
fn port_bits(
    ctx: &LowerContext<'_, '_>,
    net: &SignalSymbol,
    port: &PortInstance,
    diagnostics: &mut Diagnostics,
) -> Result<Option<SignalSlice>, ()> {
    let Port { hident, b1, b2 } = &port.port;
    let Some(b1) = b1 else {
        return Ok(None);
    };

    if !net.dims.is_empty() {
        diagnostics.not_yet_implemented(TokenRange::default(), "SDF port of an array");
        return Err(());
    }

    // The bit of a declared index, as in `lower_addressing`.
    let width = net.ty.bit_length().get();
    let transform = net.transform;
    let bit = |index: &Integer| -> Option<u32> {
        let index = index.0.parse::<i64>().ok()?;
        let offset = u32::try_from(index.checked_sub(transform.lsb)?).ok()?;
        (offset < width).then(|| {
            if transform.bit_reversed {
                width - 1 - offset
            } else {
                offset
            }
        })
    };
    let (Some(bit1), Some(bit2)) = (bit(b1), bit(b2.as_ref().unwrap_or(b1))) else {
        let mut path = display_ident(&ctx.arenas.ident_table, hident);
        if let Some(hident) = &port.hident {
            path = format!("{}.{path}", display_ident(&ctx.arenas.ident_table, hident));
        }
        path = match b2 {
            None => format!("{path}[{}]", b1.0),
            Some(b2) => format!("{path}[{}:{}]", b1.0, b2.0),
        };
        diagnostics.errors.push((
            TokenRange::default(),
            LowerErrorReason::SdfPortSelectOutOfRange(path),
            Vec::new(),
        ));
        return Err(());
    };

    let slice =
        SignalSlice::new(bit1.max(bit2), bit1.min(bit2)).expect("the bits are within the net");
    Ok((slice.width().get() != width).then_some(slice))
}

/// Whether `sid` is `scope` or lies somewhere below it in the hierarchy.
fn is_within(ctx: &LowerContext<'_, '_>, sid: SymbolId, scope: SymbolId) -> bool {
    let mut sid = Some(sid);
    while let Some(s) = sid {
        if s == scope {
            return true;
        }
        sid = ctx.table[s].parent();
    }
    false
}

/// Check that the cell at `cell_sid` is an instance of the module named by the SDF `CELLTYPE`.
fn check_celltype<'a>(
    ctx: &LowerContext<'a, '_>,
    cell_sid: SymbolId,
    celltype: IdentId,
    path: impl FnOnce() -> String,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    // IEEE Std 1497-2001 (p. 19)
    //
    // """
    // If the annotator locates the region and finds that its name does not match the CELLTYPE, it
    // shall indicate an error.
    // """
    let VSymbol::Module(module) = &ctx.table[cell_sid].content else {
        diagnostics.errors.push((
            TokenRange::default(),
            LowerErrorReason::SdfNotAnInstance(path()),
            Vec::new(),
        ));
        return Err(());
    };

    if celltype != module.module {
        diagnostics.errors.push((
            TokenRange::default(),
            LowerErrorReason::SdfCelltypeMismatch(path(), celltype, module.module),
            Vec::new(),
        ));
        return Err(());
    }

    Ok(())
}

fn resolve_port_instance<'a>(
    ctx: &LowerContext<'a, '_>,
    divider: HierarchyDivider,
    scope: SymbolId,
    ident: &PortInstance,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let PortInstance { hident, port } = ident;
    let scope = match hident {
        None => scope,
        Some(hident) => resolve_ident(ctx, divider, Some(scope), hident, diagnostics)?,
    };
    // The bit or part select of the port is handled by `port_bits`.
    resolve_ident(ctx, divider, Some(scope), &port.hident, diagnostics)
}

fn parse_signed_real_number(s: &str) -> Result<f64, ()> {
    // @TODO: This is very naive.
    s.parse().map_err(|_| ())
}

fn parse_time_value(
    s: &str,
    sdf_timescale: TimeResolution,
    resolution: TimeResolution,
) -> Result<Time, ()> {
    // @TODO: This is very lossy.
    let value = parse_signed_real_number(s)?;
    let value = sdf_timescale
        .truncate_or_multiply_f64_to(value, resolution)
        .round();
    let value = value as u64;
    Ok(Time(value))
}

fn push_delval(
    delval: &DelVal,
    delays: &mut Vec<Time>,
    sdf_timescale: TimeResolution,
    resolution: TimeResolution,
) -> Result<(), ()> {
    // This should be represented as a no-override...
    let Some(value) = &delval.delay.0 else {
        todo!();
    };

    use SignedRealNumberOrRTriple as R;

    match value {
        R::SignedRealNumber(value) => {
            delays.push(parse_time_value(value.0, sdf_timescale, resolution)?);
        }
        R::RTriple(rtriple) => {
            // This should be represented as a no-override...
            let RTriple(Some(min), Some(typ), Some(max)) = rtriple else {
                todo!();
            };

            delays.push(parse_time_value(min.0, sdf_timescale, resolution)?);
            delays.push(parse_time_value(typ.0, sdf_timescale, resolution)?);
            delays.push(parse_time_value(max.0, sdf_timescale, resolution)?);
        }
    }

    Ok(())
}

fn parse_delvallist(
    list: &DelValList,
    delays: &mut Vec<Time>,
    sdf_timescale: TimeResolution,
    resolution: TimeResolution,
) -> Result<DelayPtr, ()> {
    let offset = delays.len() as u64;
    use DelValList as L;
    use DelayPtrVariant as V;

    let variant = match list {
        L::One(..) => V::One,
        L::Two(..) => V::Two,
        L::Three(..) => V::Three,
        L::Six(..) => V::Six,
        L::Twelve(..) => V::Twelve,
    };

    let mut triple_mask = 0u16;
    for (i, dv) in list.as_slice().iter().enumerate() {
        let is_triple = matches!(
            dv,
            DelVal {
                delay: RValue(Some(SignedRealNumberOrRTriple::RTriple(_))),
                ..
            }
        );
        triple_mask |= u16::from(is_triple) << i;
        push_delval(dv, delays, sdf_timescale, resolution)?;
    }

    let delay_ptr = DelayPtr::new(offset, variant, triple_mask);
    Ok(delay_ptr)
}

/// Collects the delays of `sdf` into the path delays of the driver resolution.
pub fn lower_sdf<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    sdf: &DelayFile<'_>,
    scope: Option<SymbolId>,
) -> Result<(), ()> {
    let timescale = match &sdf.header.timescale {
        // IEEE Std 1497-2001 (p. 17)
        // """
        // If the SDF file does not contain a timescale then all time values in the file shall be
        // assumed to be in nanoseconds. This has the same effect as a timescale of 1ns.
        // """
        None => TimeResolution::NS1,

        Some(timescale) => {
            let Timescale { number, unit } = timescale;
            use TimescaleNumber as N;
            let sdf_time_size = match number {
                N::N1 | N::N1_0 => TimeSize::N1,
                N::N10 | N::N10_0 => TimeSize::N10,
                N::N100 | N::N100_0 => TimeSize::N100,
            };

            use TimescaleUnit as U;
            let sdf_time_unit = match unit {
                U::Seconds => TimeUnit::Seconds,
                U::Milliseconds => TimeUnit::Milliseconds,
                U::Microseconds => TimeUnit::Microseconds,
                U::Nanoseconds => TimeUnit::Nanoseconds,
                U::Picoseconds => TimeUnit::Picoseconds,
                U::Femtoseconds => TimeUnit::Femtoseconds,
            };

            TimeResolution {
                unit: sdf_time_unit,
                size: sdf_time_size,
            }
        }
    };
    // The paths are only collected here, the driver resolution puts them in front of their
    // outputs together with the module paths of the specify blocks.
    let mut delay_values = std::mem::take(&mut mctx.driver_resolution.path_delays.delays);

    // IEEE Std 1497-2001 (p. 16)
    //
    // """
    // If the SDF file does not contain a hierarchy divider then the default hierarchy divider
    // shall be the period (‘.’).
    // """
    let divider = sdf
        .header
        .hierarchy_divider
        .unwrap_or(HierarchyDivider::Dot);

    let mut instances = Vec::<SymbolId>::new();
    let mut instance_ranges = VgHashMap::<IdentId, Range<usize>>::default();

    let mut error = false;
    for cell in &sdf.cells {
        let vogls_sdf::Cell {
            celltype,
            instance,
            timing_specs,
        } = cell;

        let cell_sid;
        let cell_sids: &[SymbolId] = match instance {
            Instance::Empty => {
                cell_sid = match (scope, ctx.table.roots()) {
                    (Some(scope), _) => scope,
                    (None, &[root]) => root,
                    (None, _) => {
                        mctx.diagnostics
                            .not_yet_implemented(TokenRange::default(), "ambiguous top-level");
                        error = true;
                        continue;
                    }
                };
                let path = || ctx.arenas.ident_table[ctx.table[cell_sid].name()].to_string();
                if check_celltype(ctx, cell_sid, *celltype, path, &mut mctx.diagnostics).is_err() {
                    error = true;
                    continue;
                }
                std::slice::from_ref(&cell_sid)
            }
            // IEEE Std 1497-2001 (p. 20)
            //
            // """
            // The second form of the cell instance can be used to associate timing data with all
            // occurrences of the specified cell type.
            // """
            Instance::Star => {
                let module_of = |sid: SymbolId| match &ctx.table[sid].content {
                    VSymbol::Module(module) => module.module,
                    _ => unreachable!(),
                };

                if instances.is_empty() {
                    instances.extend(ctx.table.symbol_id_iter().filter(|&sid| {
                        matches!(ctx.table[sid].content, VSymbol::Module(_))
                            && scope.is_none_or(|scope| is_within(ctx, sid, scope))
                    }));
                    instances.sort_unstable_by_key(|&sid| module_of(sid));

                    let mut offset = 0;
                    for group in instances.chunk_by(|&a, &b| module_of(a) == module_of(b)) {
                        instance_ranges.insert(module_of(group[0]), offset..offset + group.len());
                        offset += group.len();
                    }
                }

                instance_ranges
                    .get(celltype)
                    .map_or(&[], |range| &instances[range.clone()])
            }
            Instance::HierarchicalIdent(hident) => {
                let Ok(sid) = resolve_ident(ctx, divider, scope, hident, &mut mctx.diagnostics)
                else {
                    error = true;
                    continue;
                };
                cell_sid = sid;
                let path = || display_ident(&ctx.arenas.ident_table, hident);
                if check_celltype(ctx, cell_sid, *celltype, path, &mut mctx.diagnostics).is_err() {
                    error = true;
                    continue;
                }
                std::slice::from_ref(&cell_sid)
            }
        };

        for &cell_sid in cell_sids {
            for timing_spec in timing_specs.iter() {
                match timing_spec {
                    TimingSpec::Delay(d) => {
                        let DelaySpec { items } = d;
                        for item in items {
                            match item {
                                vogls_sdf::DelayType::Absolute(abs) => {
                                    let AbsoluteDelayType { defs } = abs;
                                    for def in defs {
                                        match def {
                                            vogls_sdf::DelayDef::IoPath(iopath) => {
                                                let IoPathDef {
                                                    port_spec,
                                                    port_instance: to,
                                                    retain_defs,
                                                    delval_list,
                                                } = iopath;

                                                if !retain_defs.is_empty() {
                                                    todo!();
                                                }

                                                let PortSpec::Instance(from) = port_spec else {
                                                    todo!();
                                                };

                                                error |= push_sdf_path(
                                                    ctx,
                                                    mctx,
                                                    divider,
                                                    cell_sid,
                                                    (from, to),
                                                    delval_list,
                                                    &mut delay_values,
                                                    timescale,
                                                )
                                                .is_err();
                                            }

                                            // @TODO: These should all be implemented in time.
                                            vogls_sdf::DelayDef::Retain(_) => todo!(),
                                            vogls_sdf::DelayDef::Cond(_) => todo!(),
                                            vogls_sdf::DelayDef::CondElse(_) => todo!(),
                                            vogls_sdf::DelayDef::Port(_) => todo!(),
                                            vogls_sdf::DelayDef::Interconnect(interconnect) => {
                                                let InterconnectDef {
                                                    from,
                                                    to,
                                                    delval_list,
                                                } = interconnect;

                                                error |= push_sdf_path(
                                                    ctx,
                                                    mctx,
                                                    divider,
                                                    cell_sid,
                                                    (from, to),
                                                    delval_list,
                                                    &mut delay_values,
                                                    timescale,
                                                )
                                                .is_err();
                                            }
                                            vogls_sdf::DelayDef::NetDelay(_) => todo!(),
                                            vogls_sdf::DelayDef::Device(_) => todo!(),
                                        }
                                    }
                                }
                                vogls_sdf::DelayType::Increment(_) => todo!(),
                                vogls_sdf::DelayType::PathPulse(_) => todo!(),
                                vogls_sdf::DelayType::PathPulseProcent(_) => todo!(),
                            }
                        }
                    }

                    // @TODO: These should all be implemented in time.
                    TimingSpec::TimingCheck(_) => {}
                    TimingSpec::TimingEnv(_) => {}
                    TimingSpec::Label(_) => {}
                }
            }
        }
    }

    mctx.driver_resolution.path_delays.delays = delay_values;
    if error {
        return Err(());
    }
    Ok(())
}
