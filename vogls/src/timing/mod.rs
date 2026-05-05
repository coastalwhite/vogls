use std::path::Path;

use hashbrown::hash_map::Entry;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::SignalKey;
use vogls_ir::token_range::TokenRange;
use vogls_sdf::{
    AbsoluteDelayType, Consume, DelaySpec, HierarchicalIdent, IoPathDef, Port, PortInstance,
    PortSpec, TimingSpec, TokenWalker,
};
use vogls_utils::{IterSliceContinguous, VgHashMap};
use vogls_verilog::lower::specify::{
    Condition, Delay, Delays, SpecifyOutput, SpecifyPath, lower_iopath,
};
use vogls_verilog::lower::{Diagnostics, LowerContext, LowerErrorReason, MutLowerContext};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TimingProperty {
    from: SignalKey,
    to: SignalKey,
}

struct TimingContent {
    delays: Delays,
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
                                            let content = TimingContent {
                                                delays: Delays::One(Delay {
                                                    min: 1,
                                                    max: 1,
                                                    typ: 1,
                                                }),
                                            };

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
                                        vogls_sdf::DelayDef::Retain(_) => {}
                                        vogls_sdf::DelayDef::Cond(_) => {}
                                        vogls_sdf::DelayDef::CondElse(_) => {}
                                        vogls_sdf::DelayDef::Port(_) => {}
                                        vogls_sdf::DelayDef::Interconnect(_) => {}
                                        vogls_sdf::DelayDef::NetDelay(_) => {}
                                        vogls_sdf::DelayDef::Device(_) => {}
                                    }
                                }
                            }
                            vogls_sdf::DelayType::Increment(_) => todo!(),
                            vogls_sdf::DelayType::PathPulse(_) => {}
                            vogls_sdf::DelayType::PathPulseProcent(_) => {}
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
            paths.push((
                p.from,
                vec![SpecifyPath {
                    condition: Condition::None,
                    delays: c.delays.clone(),
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
