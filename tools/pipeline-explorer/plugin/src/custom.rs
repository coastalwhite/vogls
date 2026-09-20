//! Mnemonics a plugin's design can accept on top of the base ISA.
//!
//! The assembler the explorer uses ([`trva`]) dispatches mnemonics through a
//! table a plugin can add to, and this module holds the entries this repo ships:
//!
//! | mnemonic | assembles to |
//! | --- | --- |
//! | `square xd, xs` | `mul xd, xs, xs` |
//! | `l1 xd` | `li xd, 1`, i.e. `addi xd, zero, 1` |
//!
//! Both are shorthands rather than new opcodes: they encode as instructions the
//! design already runs, so nothing in the RTL has to know about them and they
//! behave exactly like what they stand for -- including `square` needing the M
//! extension, because `mul` does.
//!
//! A plugin opts in where it assembles:
//!
//! ```ignore
//! use pipeline_explorer_plugin::custom;
//!
//! let trace = get_hazard3_trace(assembly, num_cycles, &config, custom::ALL)?;
//! ```
//!
//! Nothing registers them behind a plugin's back: a design that does not want a
//! mnemonic keeps rejecting it, so the user sees the assembler refuse it rather
//! than wonder which instruction ran.
//!
//! Because they assemble to ordinary instructions, a listing shows what they
//! became -- `square a0, a1` reads back as `mul a0,a1,a1` -- exactly as the
//! pseudo-instructions the base ISA already has do.

use trva::CustomInstruction;
use trva::encoding::{Addi, Mul, XRegIdent};

/// `square xd, xs` -- `xd = xs * xs`, as `mul xd, xs, xs`.
pub const SQUARE: CustomInstruction =
    CustomInstruction::rd_rs("square", |rd, rs| Mul::new(rd, rs, rs).encode_as_u32());

/// `l1 xd` -- "load immediate 1": `xd = 1`, as `addi xd, zero, 1`.
pub const L1: CustomInstruction =
    CustomInstruction::rd("l1", |rd| Addi::new(rd, XRegIdent::Zero, 1).encode_as_u32());

/// Every mnemonic this module defines, for handing to
/// [`with_custom_instructions`].
///
/// [`with_custom_instructions`]: trva::Assembler::with_custom_instructions
pub const ALL: &[CustomInstruction] = &[SQUARE, L1];

/// The mnemonics of [`ALL`], for a manifest's `instructions` list.
pub const MNEMONICS: &[&str] = &[SQUARE.mnemonic, L1.mnemonic];

#[cfg(test)]
mod tests {
    use trva::isa::{ExtensionSet, Isa, XLen};
    use trva::{Assembler, SectionPositions};

    use super::*;

    const RV32IM: Isa = Isa {
        exts: ExtensionSet::INTEGER_MULDIV,
        xlen: XLen::Rv32,
    };

    fn assemble(source: &str, custom: &[CustomInstruction]) -> Result<Box<[u8]>, String> {
        let positions = SectionPositions {
            text: 0,
            data: 0,
            rodata: 0,
            bss: 0,
        };
        Ok(Assembler::new(RV32IM, positions)
            .with_custom_instructions(custom)
            .with_source(source)
            .map_err(|err| err.to_string())?
            .assemble()
            .text)
    }

    #[test]
    fn they_assemble_to_the_instructions_they_stand_for() {
        assert_eq!(
            assemble("square a0, a1\nl1 a2\n", ALL).unwrap(),
            assemble("mul a0, a1, a1\naddi a2, zero, 1\n", &[]).unwrap(),
        );
    }

    #[test]
    fn a_design_that_does_not_register_them_rejects_them() {
        // Assembly is what fails, rather than the program quietly meaning
        // something else on a design that never asked for the shorthand.
        let err = assemble("square a0, a1\n", &[]).unwrap_err();
        assert!(err.contains("unknown mnemonic"), "{err}");
        assert!(assemble("l1 a0\n", &[]).is_err());
    }

    #[test]
    fn mnemonics_lists_every_instruction() {
        let mnemonics: Vec<&str> = ALL.iter().map(|i| i.mnemonic).collect();
        assert_eq!(mnemonics, MNEMONICS);
    }
}
