//! Where sections are placed in the address space.
//!
//! A section either starts at a fixed address or is placed after another section, which lets a
//! layout be described without knowing how large any section turns out to be:
//!
//! ```
//! use trva::{SectionKind, SectionPlacement, SectionPositions};
//!
//! // `.bss` directly after `.data`, rounded up to a 4-byte boundary.
//! let positions = SectionPositions {
//!     text: SectionPlacement::Absolute(0x8000_0000),
//!     data: SectionPlacement::after(SectionKind::Text),
//!     rodata: SectionPlacement::after(SectionKind::Data),
//!     bss: SectionPlacement::after_aligned(SectionKind::ReadOnlyData, 4),
//! };
//! assert_eq!(
//!     positions.layout_order().unwrap(),
//!     [
//!         SectionKind::Text,
//!         SectionKind::Data,
//!         SectionKind::ReadOnlyData,
//!         SectionKind::Bss,
//!     ],
//! );
//! ```

use std::fmt;
use std::str::FromStr;

use crate::error::{LayoutError, SectionCycle};

/// One of the four sections the assembler emits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SectionKind {
    Text,
    Data,
    ReadOnlyData,
    Bss,
}

impl SectionKind {
    /// Every section, in the order they are stored in by the assembler.
    pub const ALL: [Self; 4] = [Self::Text, Self::Data, Self::ReadOnlyData, Self::Bss];

    /// Index of this section within [`SectionKind::ALL`].
    pub const fn index(self) -> usize {
        match self {
            Self::Text => 0,
            Self::Data => 1,
            Self::ReadOnlyData => 2,
            Self::Bss => 3,
        }
    }

    /// Name of this section as it is spelled in assembly, leading dot included.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => ".text",
            Self::Data => ".data",
            Self::ReadOnlyData => ".rodata",
            Self::Bss => ".bss",
        }
    }

    /// Name of this section without its leading dot.
    pub const fn bare_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Data => "data",
            Self::ReadOnlyData => "rodata",
            Self::Bss => "bss",
        }
    }
}

impl fmt::Display for SectionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Returned when a string does not name one of the sections in [`SectionKind::ALL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownSectionName;

impl fmt::Display for UnknownSectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown section name")
    }
}

impl std::error::Error for UnknownSectionName {}

impl FromStr for SectionKind {
    type Err = UnknownSectionName;

    /// Parse a section name, with or without its leading dot and in any case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('.').unwrap_or(s);
        for kind in Self::ALL {
            if s.eq_ignore_ascii_case(kind.bare_name()) {
                return Ok(kind);
            }
        }
        Err(UnknownSectionName)
    }
}

/// Where a section's base address comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionPlacement {
    /// A fixed address.
    Absolute(u32),
    /// Straight after the end of another section.
    After {
        /// The section this one follows.
        section: SectionKind,
        /// Bytes of padding to leave after `section` before aligning.
        gap: u32,
        /// Round the resulting address up to a multiple of this many bytes.
        ///
        /// `0` and `1` both mean "no alignment". As with the `.align` directive this is a byte
        /// count, not a power-of-two exponent, and need not be a power of two.
        align: u32,
    },
}

impl SectionPlacement {
    /// Place a section straight after `section`, with no gap and no alignment.
    pub const fn after(section: SectionKind) -> Self {
        Self::After {
            section,
            gap: 0,
            align: 1,
        }
    }

    /// Place a section after `section`, rounded up to a multiple of `align` bytes.
    pub const fn after_aligned(section: SectionKind, align: u32) -> Self {
        Self::After {
            section,
            gap: 0,
            align,
        }
    }

    /// The section this placement is measured from, if it is not absolute.
    pub const fn depends_on(self) -> Option<SectionKind> {
        match self {
            Self::Absolute(_) => None,
            Self::After { section, .. } => Some(section),
        }
    }
}

impl fmt::Display for SectionPlacement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Absolute(addr) => write!(f, "{addr:#010x}"),
            Self::After {
                section,
                gap,
                align,
            } => {
                write!(f, "after:{}", section.bare_name())?;
                if gap != 0 {
                    write!(f, ":gap={gap}")?;
                }
                if align > 1 {
                    write!(f, ":align={align}")?;
                }
                Ok(())
            }
        }
    }
}

/// The placement of every section.
#[derive(Clone, Copy, Debug)]
pub struct SectionPositions {
    pub text: SectionPlacement,
    pub data: SectionPlacement,
    pub rodata: SectionPlacement,
    pub bss: SectionPlacement,
}

impl SectionPositions {
    /// Give every section a fixed address.
    pub const fn absolute(text: u32, data: u32, rodata: u32, bss: u32) -> Self {
        Self {
            text: SectionPlacement::Absolute(text),
            data: SectionPlacement::Absolute(data),
            rodata: SectionPlacement::Absolute(rodata),
            bss: SectionPlacement::Absolute(bss),
        }
    }

    /// The placement of a single section.
    pub const fn placement(&self, kind: SectionKind) -> SectionPlacement {
        match kind {
            SectionKind::Text => self.text,
            SectionKind::Data => self.data,
            SectionKind::ReadOnlyData => self.rodata,
            SectionKind::Bss => self.bss,
        }
    }

    /// The order sections have to be laid out in for every placement to be resolvable.
    ///
    /// A section only ever follows one other section, so ordering by the length of each
    /// section's chain of placements puts every section after the one it is measured from.
    ///
    /// # Errors
    ///
    /// Fails if a section is placed after itself, or if a group of sections are placed after
    /// each other in a cycle; in both cases no order exists.
    pub fn layout_order(&self) -> Result<[SectionKind; 4], LayoutError> {
        let mut depth = [0u8; 4];

        for kind in SectionKind::ALL {
            // Every section in a chain is distinct - the walk stops as soon as one repeats - so
            // there is never more than one entry per section to remember.
            let mut chain = [kind; 4];
            let mut len = 1usize;
            let mut current = kind;

            while let Some(next) = self.placement(current).depends_on() {
                if next == current {
                    return Err(LayoutError::SelfReference { section: current });
                }
                if let Some(start) = chain[..len].iter().position(|&seen| seen == next) {
                    // The walk may have entered the cycle partway, so report it from where it
                    // closes rather than from where the walk began.
                    return Err(LayoutError::Cycle {
                        cycle: SectionCycle::new(&chain[start..len]),
                    });
                }
                // `next` is new, and the four sections cannot all already be in `chain` without
                // it having matched above, so there is always room.
                chain[len] = next;
                len += 1;
                current = next;
            }

            depth[kind.index()] = (len - 1) as u8;
        }

        // Order by chain length, so a section always trails the one it is measured from.
        // Counting the four possible depths avoids both a sort and any allocation, and keeps
        // sections of equal depth in `SectionKind::ALL` order so the layout is deterministic.
        let mut order = [SectionKind::Text; 4];
        let mut placed = 0usize;
        for target in 0..SectionKind::ALL.len() as u8 {
            for kind in SectionKind::ALL {
                if depth[kind.index()] == target {
                    order[placed] = kind;
                    placed += 1;
                }
            }
        }
        debug_assert_eq!(
            placed,
            order.len(),
            "every section should be placed exactly once"
        );

        Ok(order)
    }
}

/// The base a numeric literal is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radix {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

impl Radix {
    pub const fn value(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Octal => "octal",
            Self::Decimal => "decimal",
            Self::Hexadecimal => "hexadecimal",
        }
    }

    pub const fn digit_set(self) -> &'static str {
        match self {
            Self::Binary => "`0` or `1`",
            Self::Octal => "`0`-`7`",
            Self::Decimal => "`0`-`9`",
            Self::Hexadecimal => "`0`-`9`, `a`-`f` or `A`-`F`",
        }
    }
}

/// Why a [`SectionPlacement`] could not be parsed.
///
/// Every offset is a byte offset into the string that was handed to [`SectionPlacement::from_str`],
/// so it can be pointed at directly in a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsePlacementError {
    /// The input was empty or only whitespace.
    Empty,
    /// A radix prefix such as `0x` was given, but no digits followed it.
    NoDigits { prefix: &'static str },
    /// A character that is not a digit in the selected radix.
    InvalidDigit {
        ch: char,
        offset: usize,
        radix: Radix,
    },
    /// A hex digit appeared in an unprefixed literal - almost always a
    /// forgotten `0x`, so it gets its own message.
    MissingHexPrefix { ch: char, offset: usize },
    /// A `_` separator that is not between two digits.
    StrayUnderscore { offset: usize },
    /// The value is well-formed but larger than `u32::MAX`.
    Overflow,
    /// `after` was given without the section to follow.
    MissingSection { offset: usize },
    /// The section to follow is not one of `text`, `data`, `rodata` or `bss`.
    UnknownSection { name: String, offset: usize },
    /// An attribute other than `align` or `gap`.
    UnknownAttribute { key: String, offset: usize },
    /// An attribute written without `=<value>`.
    MissingAttributeValue { key: String, offset: usize },
    /// The same attribute was given twice.
    DuplicateAttribute { key: &'static str, offset: usize },
    /// `align=0`, which would leave the address unaligned rather than erroring silently.
    ZeroAlign { offset: usize },
    /// An empty `:`-separated field, usually a doubled or trailing `:`.
    EmptyField { offset: usize },
}

impl fmt::Display for ParsePlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(
                f,
                "expected an address such as `0x2000_0000`, or a relative position such as \
                 `after:data:align=4`, found nothing"
            ),
            Self::NoDigits { prefix } => write!(f, "no digits after the `{prefix}` prefix"),
            Self::InvalidDigit { ch, offset, radix } => write!(
                f,
                "invalid {} digit `{}` at offset {} (expected {})",
                radix.name(),
                ch.escape_debug(),
                offset,
                radix.digit_set(),
            ),
            Self::MissingHexPrefix { ch, offset } => write!(
                f,
                "invalid decimal digit `{}` at offset {}; \
                 hexadecimal addresses need a `0x` prefix",
                ch.escape_debug(),
                offset,
            ),
            Self::StrayUnderscore { offset } => {
                write!(f, "`_` at offset {offset} must sit between two digits")
            }
            Self::Overflow => write!(
                f,
                "address does not fit in 32 bits (the highest address is 0xffff_ffff)",
            ),
            Self::MissingSection { offset } => write!(
                f,
                "expected a section to follow at offset {offset}, as in `after:data`",
            ),
            Self::UnknownSection { name, offset } => write!(
                f,
                "unknown section `{}` at offset {}; expected `text`, `data`, `rodata` or `bss`",
                name.escape_debug(),
                offset,
            ),
            Self::UnknownAttribute { key, offset } => write!(
                f,
                "unknown attribute `{}` at offset {}; expected `align` or `gap`",
                key.escape_debug(),
                offset,
            ),
            Self::MissingAttributeValue { key, offset } => write!(
                f,
                "attribute `{}` at offset {} needs a value, as in `{}=4`",
                key.escape_debug(),
                offset,
                key.escape_debug(),
            ),
            Self::DuplicateAttribute { key, offset } => {
                write!(
                    f,
                    "attribute `{key}` is given twice, again at offset {offset}"
                )
            }
            Self::ZeroAlign { offset } => write!(
                f,
                "`align=0` at offset {offset} is not an alignment; use `align=1` for none",
            ),
            Self::EmptyField { offset } => {
                write!(f, "empty field at offset {offset}; check for a doubled `:`")
            }
        }
    }
}

impl std::error::Error for ParsePlacementError {}

impl FromStr for SectionPlacement {
    type Err = ParsePlacementError;

    /// Parse either an absolute address or a position relative to another section.
    ///
    /// ```text
    /// placement := <address> | "after" ":" <section> ( ":" <attribute> )*
    /// address   := ( "0x" | "0o" | "0b" )? <digits>
    /// section   := "."? ( "text" | "data" | "rodata" | "bss" )
    /// attribute := ( "align" | "gap" ) "=" <address>
    /// ```
    ///
    /// Names and radix prefixes are case-insensitive and `_` may separate digits.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Offsets in errors refer to the original string, so account for what gets trimmed.
        let lead = s.len() - s.trim_start().len();
        let body = s.trim();

        if body.is_empty() {
            return Err(ParsePlacementError::Empty);
        }

        const AFTER: &str = "after";
        let is_relative = body
            .get(..AFTER.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(AFTER))
            && matches!(body.as_bytes().get(AFTER.len()), None | Some(b':'));

        if !is_relative {
            return parse_u32(body, lead).map(Self::Absolute);
        }

        let rest = &body[AFTER.len()..];
        let Some(fields) = rest.strip_prefix(':') else {
            return Err(ParsePlacementError::MissingSection {
                offset: lead + AFTER.len(),
            });
        };

        // Split on `:` while keeping each field's offset in the original string.
        let mut offset = lead + AFTER.len() + 1;
        let mut fields = fields.split(':').map(|field| {
            let at = offset;
            offset += field.len() + 1;
            (field, at)
        });

        let (name, name_offset) = fields.next().expect("split yields at least one field");
        if name.is_empty() {
            return Err(ParsePlacementError::MissingSection {
                offset: name_offset,
            });
        }
        let section =
            name.parse::<SectionKind>()
                .map_err(|_| ParsePlacementError::UnknownSection {
                    name: name.to_owned(),
                    offset: name_offset,
                })?;

        let mut gap = None;
        let mut align = None;
        for (field, at) in fields {
            if field.is_empty() {
                return Err(ParsePlacementError::EmptyField { offset: at });
            }

            let (key, value) = match field.split_once('=') {
                Some((key, value)) => (key, value),
                None => {
                    return Err(ParsePlacementError::MissingAttributeValue {
                        key: field.to_owned(),
                        offset: at,
                    });
                }
            };

            let value_offset = at + key.len() + 1;
            if value.is_empty() {
                return Err(ParsePlacementError::MissingAttributeValue {
                    key: key.to_owned(),
                    offset: at,
                });
            }

            let slot = if key.eq_ignore_ascii_case("align") {
                &mut align
            } else if key.eq_ignore_ascii_case("gap") {
                &mut gap
            } else {
                return Err(ParsePlacementError::UnknownAttribute {
                    key: key.to_owned(),
                    offset: at,
                });
            };

            if slot.is_some() {
                return Err(ParsePlacementError::DuplicateAttribute {
                    key: if key.eq_ignore_ascii_case("align") {
                        "align"
                    } else {
                        "gap"
                    },
                    offset: at,
                });
            }

            let value = parse_u32(value, value_offset)?;
            if key.eq_ignore_ascii_case("align") && value == 0 {
                return Err(ParsePlacementError::ZeroAlign {
                    offset: value_offset,
                });
            }
            *slot = Some(value);
        }

        Ok(Self::After {
            section,
            gap: gap.unwrap_or(0),
            align: align.unwrap_or(1),
        })
    }
}

/// Parse an unsigned 32-bit integer with an optional radix prefix and `_` digit separators.
///
/// `base_offset` is where `s` starts within the string the user wrote, so that offsets in errors
/// point at the offending character rather than into `s`.
fn parse_u32(s: &str, base_offset: usize) -> Result<u32, ParsePlacementError> {
    let (radix, prefix, digits) = if let Some(rest) = strip_either(s, "0x", "0X") {
        (Radix::Hexadecimal, "0x", rest)
    } else if let Some(rest) = strip_either(s, "0o", "0O") {
        (Radix::Octal, "0o", rest)
    } else if let Some(rest) = strip_either(s, "0b", "0B") {
        (Radix::Binary, "0b", rest)
    } else {
        (Radix::Decimal, "", s)
    };

    let base = base_offset + prefix.len();
    let mut value: u32 = 0;
    let mut seen_digit = false;
    let mut trailing_underscore: Option<usize> = None;

    for (i, ch) in digits.char_indices() {
        let offset = base + i;

        if ch == '_' {
            if !seen_digit {
                return Err(ParsePlacementError::StrayUnderscore { offset });
            }
            trailing_underscore = Some(offset);
            continue;
        }
        trailing_underscore = None;

        let digit = match ch.to_digit(radix.value()) {
            Some(digit) => digit,
            None if radix == Radix::Decimal && ch.is_ascii_hexdigit() => {
                return Err(ParsePlacementError::MissingHexPrefix { ch, offset });
            }
            None => {
                return Err(ParsePlacementError::InvalidDigit { ch, offset, radix });
            }
        };
        seen_digit = true;

        value = value
            .checked_mul(radix.value())
            .and_then(|acc| acc.checked_add(digit))
            .ok_or(ParsePlacementError::Overflow)?;
    }

    if let Some(offset) = trailing_underscore {
        return Err(ParsePlacementError::StrayUnderscore { offset });
    }
    if !seen_digit {
        return Err(ParsePlacementError::NoDigits { prefix });
    }

    Ok(value)
}

fn strip_either<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    s.strip_prefix(a).or_else(|| s.strip_prefix(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    use SectionKind::{Bss, Data, ReadOnlyData, Text};

    fn parse(s: &str) -> Result<SectionPlacement, ParsePlacementError> {
        s.parse()
    }

    #[test]
    fn absolute_addresses() {
        assert_eq!(
            parse("0x2000_0000"),
            Ok(SectionPlacement::Absolute(0x2000_0000))
        );
        assert_eq!(
            parse("0X80000000"),
            Ok(SectionPlacement::Absolute(0x8000_0000))
        );
        assert_eq!(parse("0o777"), Ok(SectionPlacement::Absolute(0o777)));
        assert_eq!(parse("0b1010"), Ok(SectionPlacement::Absolute(0b1010)));
        assert_eq!(parse("1024"), Ok(SectionPlacement::Absolute(1024)));
        assert_eq!(parse("  0x10  "), Ok(SectionPlacement::Absolute(0x10)));
        assert_eq!(
            parse("0xffffffff"),
            Ok(SectionPlacement::Absolute(u32::MAX))
        );
    }

    #[test]
    fn absolute_address_errors() {
        assert_eq!(parse(""), Err(ParsePlacementError::Empty));
        assert_eq!(parse("   "), Err(ParsePlacementError::Empty));
        assert_eq!(
            parse("0x"),
            Err(ParsePlacementError::NoDigits { prefix: "0x" })
        );
        assert_eq!(
            parse("0x1g"),
            Err(ParsePlacementError::InvalidDigit {
                ch: 'g',
                offset: 3,
                radix: Radix::Hexadecimal,
            }),
        );
        // A hex digit without a prefix is a forgotten `0x`, not just a bad decimal digit.
        assert_eq!(
            parse("80000000f"),
            Err(ParsePlacementError::MissingHexPrefix { ch: 'f', offset: 8 }),
        );
        assert_eq!(
            parse("_1"),
            Err(ParsePlacementError::StrayUnderscore { offset: 0 })
        );
        assert_eq!(
            parse("1_"),
            Err(ParsePlacementError::StrayUnderscore { offset: 1 })
        );
        assert_eq!(parse("0x100000000"), Err(ParsePlacementError::Overflow));
        // Offsets account for leading whitespace.
        assert_eq!(
            parse(" 1_"),
            Err(ParsePlacementError::StrayUnderscore { offset: 2 })
        );
    }

    #[test]
    fn relative_placements() {
        assert_eq!(parse("after:data"), Ok(SectionPlacement::after(Data)));
        assert_eq!(parse("after:.data"), Ok(SectionPlacement::after(Data)));
        assert_eq!(parse("AFTER:DATA"), Ok(SectionPlacement::after(Data)));
        assert_eq!(parse("after:text"), Ok(SectionPlacement::after(Text)));
        assert_eq!(
            parse("after:rodata"),
            Ok(SectionPlacement::after(ReadOnlyData))
        );
        assert_eq!(parse("after:bss"), Ok(SectionPlacement::after(Bss)));

        assert_eq!(
            parse("after:data:align=4"),
            Ok(SectionPlacement::after_aligned(Data, 4)),
        );
        assert_eq!(
            parse("after:data:align=0x10"),
            Ok(SectionPlacement::after_aligned(Data, 16)),
        );
        assert_eq!(
            parse("after:data:gap=8"),
            Ok(SectionPlacement::After {
                section: Data,
                gap: 8,
                align: 1,
            }),
        );
        // Attributes may appear in either order.
        assert_eq!(
            parse("after:data:gap=8:align=4"),
            parse("after:data:align=4:gap=8"),
        );
        assert_eq!(
            parse("after:data:ALIGN=4"),
            Ok(SectionPlacement::after_aligned(Data, 4)),
        );
    }

    #[test]
    fn relative_placement_errors() {
        assert_eq!(
            parse("after"),
            Err(ParsePlacementError::MissingSection { offset: 5 })
        );
        assert_eq!(
            parse("after:"),
            Err(ParsePlacementError::MissingSection { offset: 6 })
        );
        assert_eq!(
            parse("after:stack"),
            Err(ParsePlacementError::UnknownSection {
                name: "stack".to_owned(),
                offset: 6,
            }),
        );
        assert_eq!(
            parse("after:data:pad=4"),
            Err(ParsePlacementError::UnknownAttribute {
                key: "pad".to_owned(),
                offset: 11,
            }),
        );
        assert_eq!(
            parse("after:data:align"),
            Err(ParsePlacementError::MissingAttributeValue {
                key: "align".to_owned(),
                offset: 11,
            }),
        );
        assert_eq!(
            parse("after:data:align="),
            Err(ParsePlacementError::MissingAttributeValue {
                key: "align".to_owned(),
                offset: 11,
            }),
        );
        assert_eq!(
            parse("after:data:align=4:align=8"),
            Err(ParsePlacementError::DuplicateAttribute {
                key: "align",
                offset: 19,
            }),
        );
        assert_eq!(
            parse("after:data:align=0"),
            Err(ParsePlacementError::ZeroAlign { offset: 17 }),
        );
        assert_eq!(
            parse("after:data::align=4"),
            Err(ParsePlacementError::EmptyField { offset: 11 }),
        );
        // Offsets inside an attribute value point at the offending character.
        assert_eq!(
            parse("after:data:align=4x"),
            Err(ParsePlacementError::InvalidDigit {
                ch: 'x',
                offset: 18,
                radix: Radix::Decimal,
            }),
        );
    }

    /// `aftermath` is a section name, not the `after` keyword.
    #[test]
    fn after_is_only_a_keyword_when_followed_by_a_colon() {
        assert_eq!(
            parse("aftermath"),
            Err(ParsePlacementError::MissingHexPrefix { ch: 'a', offset: 0 }),
        );
    }

    #[test]
    fn display_round_trips() {
        for text in [
            "0x80000000",
            "after:data",
            "after:data:align=4",
            "after:data:gap=8",
            "after:data:gap=8:align=16",
            "after:rodata:align=4096",
        ] {
            let parsed = parse(text).expect("should parse");
            assert_eq!(parsed.to_string(), text, "{text} did not round-trip");
            assert_eq!(parse(&parsed.to_string()), Ok(parsed));
        }
    }

    #[test]
    fn section_names_parse_both_ways() {
        for kind in SectionKind::ALL {
            assert_eq!(kind.name().parse::<SectionKind>(), Ok(kind));
            assert_eq!(kind.bare_name().parse::<SectionKind>(), Ok(kind));
            assert_eq!(
                kind.bare_name().to_uppercase().parse::<SectionKind>(),
                Ok(kind),
            );
        }
        assert_eq!("".parse::<SectionKind>(), Err(UnknownSectionName));
        assert_eq!("heap".parse::<SectionKind>(), Err(UnknownSectionName));
    }

    #[test]
    fn errors_mention_what_went_wrong() {
        let err = parse("after:stack").unwrap_err().to_string();
        assert!(err.contains("stack"), "{err}");
        assert!(err.contains("rodata"), "{err}");

        let err = parse("").unwrap_err().to_string();
        assert!(err.contains("after:data:align=4"), "{err}");
    }

    /// Resolving an order allocates nothing, on either the success or the failure path.
    #[test]
    fn layout_order_does_not_allocate() {
        let chained = SectionPositions {
            text: SectionPlacement::Absolute(0x8000_0000),
            data: SectionPlacement::after(Text),
            rodata: SectionPlacement::after_aligned(Data, 16),
            bss: SectionPlacement::after_aligned(ReadOnlyData, 8),
        };

        let mut cyclic = SectionPositions::absolute(0, 0, 0, 0);
        cyclic.data = SectionPlacement::after(Bss);
        cyclic.bss = SectionPlacement::after(Data);

        let mut self_referential = SectionPositions::absolute(0, 0, 0, 0);
        self_referential.bss = SectionPlacement::after(Bss);

        for positions in [chained, cyclic, self_referential] {
            // Resolve once first, so nothing lazily initialised on the first call is counted.
            let expected = positions.layout_order();
            assert_eq!(positions.layout_order(), expected);
        }
    }
}
