use std::fmt;

pub type AssembleResult<T> = Result<T, Box<AssembleError>>;

#[derive(Debug, Clone)]
pub struct AssembleError {
    pub(crate) reason: std::borrow::Cow<'static, str>,
    pub(crate) line: usize,
    pub(crate) line_offset: usize,
    pub(crate) _offset: usize,
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]: {}", self.line, self.line_offset, self.reason)
    }
}
impl std::error::Error for AssembleError {}

/// A group of sections placed after each other in a cycle.
///
/// Holds each section in the cycle once, in placement order; displaying it closes the loop back
/// onto the section it starts from. There are only four sections, so this needs no allocation.
#[derive(Clone, Copy)]
pub struct SectionCycle {
    sections: [crate::SectionKind; 4],
    len: u8,
}

impl SectionCycle {
    pub(crate) fn new(sections: &[crate::SectionKind]) -> Self {
        debug_assert!(!sections.is_empty() && sections.len() <= 4);

        let mut buffer = [crate::SectionKind::Text; 4];
        buffer[..sections.len()].copy_from_slice(sections);
        Self {
            sections: buffer,
            len: sections.len() as u8,
        }
    }

    /// The sections in the cycle, each once, in placement order.
    pub fn sections(&self) -> &[crate::SectionKind] {
        &self.sections[..self.len as usize]
    }
}

// Only the occupied prefix is meaningful, so the unused tail must not take part in comparison or
// in the debug output.
impl PartialEq for SectionCycle {
    fn eq(&self, other: &Self) -> bool {
        self.sections() == other.sections()
    }
}
impl Eq for SectionCycle {}

impl fmt::Debug for SectionCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.sections()).finish()
    }
}

impl fmt::Display for SectionCycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for section in self.sections() {
            write!(f, "{section} -> ")?;
        }
        match self.sections().first() {
            Some(first) => write!(f, "{first}"),
            None => Ok(()),
        }
    }
}

/// A section layout that cannot be resolved into addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutError {
    /// A section is placed after itself.
    SelfReference { section: crate::SectionKind },
    /// A group of sections are placed after each other in a cycle.
    Cycle { cycle: SectionCycle },
    /// A section starts or ends beyond the end of the 32-bit address space.
    AddressOverflow { section: crate::SectionKind },
    /// A compressed branch or jump cannot reach its target: it is further
    /// away than the instruction encodes, or not 2-byte aligned. Unlike the
    /// full-size instructions, these are never turned into longer sequences.
    TargetOutOfRange {
        mnemonic: &'static str,
        offset: i64,
        min: i32,
        max: i32,
    },
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfReference { section } => {
                write!(f, "{section} cannot be placed after itself")
            }
            Self::Cycle { cycle } => {
                write!(f, "section placements form a cycle: {cycle}")
            }
            Self::AddressOverflow { section } => write!(
                f,
                "{section} does not fit in the 32-bit address space \
                 (the highest address is 0xffff_ffff)",
            ),
            Self::TargetOutOfRange {
                mnemonic,
                offset,
                min,
                max,
            } => write!(
                f,
                "the target of '{mnemonic}' is {offset} bytes away, but it can only reach an even \
                 offset in {min}..={max}",
            ),
        }
    }
}

impl std::error::Error for LayoutError {}
