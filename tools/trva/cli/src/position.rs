//! The `--text`/`--data`/`--rodata`/`--bss` argument type.
//!
//! The grammar and its diagnostics live in [`trva::SectionPlacement`]; this is the clap-facing
//! wrapper around it, so that the assembler and this CLI cannot drift apart on what a position
//! looks like.

use std::fmt;
use std::str::FromStr;

use trva::{ParsePlacementError, SectionPlacement};

/// Where one section starts: a fixed address, or a position relative to another section.
#[derive(Clone, Copy, Debug)]
pub struct SectionPosition(pub SectionPlacement);

impl fmt::Display for SectionPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SectionPosition {
    type Err = ParsePlacementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use trva::SectionKind;

    #[test]
    fn parses_both_forms() {
        assert_eq!(
            "0x8000_0000".parse::<SectionPosition>().unwrap().0,
            SectionPlacement::Absolute(0x8000_0000),
        );
        assert_eq!(
            "after:data:align=4".parse::<SectionPosition>().unwrap().0,
            SectionPlacement::after_aligned(SectionKind::Data, 4),
        );
    }

    #[test]
    fn displays_what_it_parsed() {
        for text in ["0x80000000", "after:data:align=4"] {
            assert_eq!(text.parse::<SectionPosition>().unwrap().to_string(), text);
        }
    }

    #[test]
    fn reports_the_underlying_error() {
        let err = "after:stack".parse::<SectionPosition>().unwrap_err();
        assert!(err.to_string().contains("stack"), "{err}");
    }
}
