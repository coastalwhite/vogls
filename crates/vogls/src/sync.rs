//! Contains Sync wrappers for the Design API.

use std::sync::{Arc, Mutex};

use vogls_ir::LogicMode;
use vogls_verilog::arena::Arena;

use crate::{LoweredDesign, VoglsPlugin};

#[derive(Clone)]
pub struct ParsedDesign {
    arena: Arc<Mutex<Arena>>,
    design: crate::ParsedDesign<'static>,
}
impl ParsedDesign {
    pub fn parse(builder: crate::DesignBuilder) -> Result<Self, Box<crate::ParseError>> {
        let arena = Arena::new();
        let design = builder.parse(&arena)?;
        // SAFETY:
        // Design only borrows from `arena`, which has stable allocations and is kept
        // alive.
        //
        // NOTE: Drop order needs to guaranteed here, @TODO?
        let design = unsafe {
            std::mem::transmute::<crate::ParsedDesign<'_>, crate::ParsedDesign<'static>>(design)
        };
        let arena = Arc::new(std::sync::Mutex::new(arena));
        Ok(Self { arena, design })
    }

    pub fn elaborate(
        self,
        mode: LogicMode,
        top_level_module: Option<String>,
    ) -> Result<ElaboratedDesign, ()> {
        let Self { arena, design } = self;
        let design = design
            .elaborate(mode, top_level_module)
            .map_err(|_| ())?;
        let design = ElaboratedDesign { arena, design };
        Ok(design)
    }
}

#[derive(Clone)]
pub struct ElaboratedDesign {
    // Kept around for ElaboratedDesign references to be sound.
    #[expect(unused)]
    arena: Arc<Mutex<Arena>>,

    design: crate::ElaboratedDesign<'static>,
}

impl ElaboratedDesign {
    pub fn lower(self, plugins: Vec<Box<dyn VoglsPlugin>>) -> Result<LoweredDesign, ()> {
        self.design.lower(plugins).map_err(|_| ())
    }
}
