use std::path::Path;

use hashbrown::hash_map::Entry;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::SignalKey;
use vogls_ir::token_range::TokenRange;
use vogls_sdf::{
    AbsoluteDelayType, Consume, DelVal, DelValList, DelaySpec, HierarchicalIdent, IoPathDef, Port,
    PortInstance, PortSpec, RTriple, RValue, SignedRealNumberOrRTriple, TimingSpec, TokenWalker,
};
use vogls_utils::{IterSliceContinguous, VgHashMap};
use vogls_verilog::lower::specify::{Condition, SpecifyOutput, SpecifyPath, lower_iopath};
use vogls_verilog::lower::{Diagnostics, LowerContext, LowerErrorReason, MutLowerContext};

use self::delay_pointer::{DelayPtr, DelayPtrVariant};

mod delay_pointer;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TimingProperty {
    from: SignalKey,
    to: SignalKey,
}

struct TimingContent {
    delays: DelayPtr,
}

fn resolve_ident<'a>(
    ctx: &LowerContext<'a>,
    root: Option<SymbolId>,
    ident: &HierarchicalIdent,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    macro_rules! unknown_ident {
        ($s:expr) => {
            diagnostics.errors.push((
                TokenRange::default(),
                LowerErrorReason::NotYetImplemented("unknown sdf module"),
                vec![format!("unknown sdf module: {}", $s).into()],
            ));
            return Err(());
        };
    }

    let Some(fst) = ctx.arenas.ident_table.get(ident.fst) else {
        unknown_ident!(ident.fst);
    };
    let cell_sid = match root {
        None => ctx.table.resolve_root(fst),
        Some(scope) => ctx.table.resolve(scope, fst),
    };
    let Some(mut cell_sid) = cell_sid else {
        unknown_ident!(ident.fst);
    };
    // @TODO: Check divider
    for (_, n) in &ident.next {
        let Some(n_iid) = ctx.arenas.ident_table.get(n) else {
            unknown_ident!(n);
        };
        let Some(new_cell_sid) = ctx.table.resolve(cell_sid, n_iid) else {
            unknown_ident!(n);
        };
        cell_sid = new_cell_sid;
    }
    Ok(cell_sid)
}

fn resolve_port_instance<'a>(
    ctx: &LowerContext<'a>,
    scope: SymbolId,
    ident: &PortInstance,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let PortInstance { hident, port } = ident;
    let scope = match hident {
        None => scope,
        Some(hident) => resolve_ident(ctx, Some(scope), hident, diagnostics)?,
    };
    let Port { hident, b1, b2 } = port;
    if b1.is_some() || b2.is_some() {
        todo!();
    }
    resolve_ident(ctx, Some(scope), hident, diagnostics)
}

fn parse_signed_real_number(s: &str) -> Result<f64, ()> {
    // @TODO: This is very naive.
    s.parse().map_err(|_| ())
}

fn parse_time_value(s: &str, timescale: u64) -> Result<u64, ()> {
    // @TODO: This is very lossy.
    let value = parse_signed_real_number(s)?;
    let timescale = timescale as f64;
    let value = (value * timescale).round();
    let value = value as u64;
    Ok(value)
}

fn push_delval(delval: &DelVal, delays: &mut Vec<u64>, timescale: u64) -> Result<(), ()> {
    // This should be represented as a no-override...
    let Some(value) = &delval.delay.0 else {
        todo!();
    };

    use SignedRealNumberOrRTriple as R;

    match value {
        R::SignedRealNumber(value) => {
            delays.push(parse_time_value(value.0, timescale)?);
        }
        R::RTriple(rtriple) => {
            // This should be represented as a no-override...
            let RTriple(Some(min), Some(typ), Some(max)) = rtriple else {
                todo!();
            };

            delays.push(parse_time_value(min.0, timescale)?);
            delays.push(parse_time_value(typ.0, timescale)?);
            delays.push(parse_time_value(max.0, timescale)?);
        }
    }

    Ok(())
}

fn parse_delvallist(
    list: &DelValList,
    delays: &mut Vec<u64>,
    timescale: u64,
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
        push_delval(dv, delays, timescale)?;
    }

    let delay_ptr = DelayPtr::new(offset, variant, triple_mask);
    Ok(delay_ptr)
}

pub fn lower_sdf<'a>(
    ctx: &mut LowerContext<'a>,
    mctx: &mut MutLowerContext,
    path: &Path,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Ok(sdf) = std::fs::read_to_string(path) else {
        diagnostics.not_yet_implemented(TokenRange::default(), "SDF file cannot be opened");
        return Err(());
    };

    let mut tkw = TokenWalker::new(&sdf);
    let sdf = match vogls_sdf::DelayFile::consume(&mut tkw) {
        Ok(sdf) => sdf,
        Err(err) => {
            diagnostics.errors.push((
                TokenRange::default(),
                LowerErrorReason::NotYetImplemented("failed to parse SDF file"),
                vec![err.to_string()],
            ));
            return Err(());
        }
    };

    let mut output_paths = VgHashMap::<SignalKey, SymbolId>::default();
    let mut properties = Vec::<(TimingProperty, TimingContent)>::new();
    let mut property_to_content = VgHashMap::<TimingProperty, usize>::default();
    let mut delays = Vec::<u64>::new();

    let mut error = false;
    for cell in sdf.cells {
        let vogls_sdf::Cell {
            celltype: _,
            instance,
            timing_specs,
        } = &cell;

        let root = None;
        let vogls_sdf::Instance::HierarchicalIdent(hident) = instance else {
            todo!("not yet implemented: non-hierarchical ident");
        };
        let Ok(cell_sid) = resolve_ident(ctx, root, &hident, diagnostics) else {
            error = true;
            continue;
        };

        // @TODO: Check celltype

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

                                            let (Ok(from_sid), Ok(to_sid)) = (
                                                resolve_port_instance(
                                                    ctx,
                                                    cell_sid,
                                                    from,
                                                    diagnostics,
                                                ),
                                                resolve_port_instance(
                                                    ctx,
                                                    cell_sid,
                                                    to,
                                                    diagnostics,
                                                ),
                                            ) else {
                                                error = true;
                                                continue;
                                            };

                                            let (
                                                vogls_verilog::elaborate::VSymbol::Net(from_net),
                                                vogls_verilog::elaborate::VSymbol::Net(to_net),
                                            ) = (
                                                &ctx.table[from_sid].content,
                                                &ctx.table[to_sid].content,
                                            )
                                            else {
                                                diagnostics.not_yet_implemented(
                                                    TokenRange::default(),
                                                    "Not a net",
                                                );
                                                error = true;
                                                continue;
                                            };

                                            let from = from_net.net.probe_signal();
                                            let to = to_net.net.blocking_drive_signal();
                                            let property = TimingProperty { from, to };
                                            let delays =
                                                parse_delvallist(delval_list, &mut delays, 1)?;
                                            let content = TimingContent { delays };

                                            output_paths.insert(to, to_sid);
                                            match property_to_content.entry(property) {
                                                Entry::Occupied(entry) => {
                                                    properties[*entry.get()].1 = content;
                                                }
                                                Entry::Vacant(entry) => {
                                                    let idx = properties.len();
                                                    properties.push((property, content));
                                                    entry.insert(idx);
                                                }
                                            }
                                        }

                                        // @TODO: These should all be implemented in time.
                                        vogls_sdf::DelayDef::Retain(_) => todo!(),
                                        vogls_sdf::DelayDef::Cond(_) => todo!(),
                                        vogls_sdf::DelayDef::CondElse(_) => todo!(),
                                        vogls_sdf::DelayDef::Port(_) => todo!(),
                                        vogls_sdf::DelayDef::Interconnect(_) => todo!(),
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

    drop(property_to_content);

    properties.sort_unstable_by_key(|(p, _)| (p.to, p.from));

    let mut input_before_lut = VgHashMap::default();
    let mut input_before = Vec::new();
    let outputs = IterSliceContinguous::new(&properties, |(p, _)| &p.to);
    for paths_to_output in outputs {
        let output = paths_to_output[0].0.to;
        let sid = output_paths[&output];
        let mut inputs = VgHashMap::default();
        let mut paths = Vec::new();

        for (p, c) in paths_to_output {
            inputs.insert(p.from, paths.len());
            let delays = c.delays.materialize(&delays);
            paths.push((
                p.from,
                vec![SpecifyPath {
                    condition: Condition::None,
                    delays,
                }],
            ));
        }

        let specify = SpecifyOutput { sid, inputs, paths };

        lower_iopath(
            ctx,
            mctx,
            sid,
            output,
            specify,
            &mut input_before_lut,
            &mut input_before,
        )?;
    }

    if error {
        return Err(());
    }

    Ok(())
}
