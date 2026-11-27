use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use vogls_ir::LogicMode;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_utils::BitsetU8;

use crate::ast::udp::{UdpEdgeIndicator, UdpEdgeSymbol, UdpLevelSymbol, UdpOutputSymbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Level {
    X = 0,
    L0 = 1,
    L1 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Transition(u16);

impl Level {
    /// Every value, at the index of its discriminant.
    pub const ALL: [Level; 3] = [Level::X, Level::L0, Level::L1];

    /// The values of a signal with `mode`.
    pub(super) fn values(mode: LogicMode) -> &'static [Level] {
        match mode {
            LogicMode::TwoValue => &[Level::L0, Level::L1],
            LogicMode::FourValue => &[Level::L0, Level::L1, Level::X],
        }
    }

    /// This value as a signal with `mode` stores it: a two-valued signal stores `x` as `0`.
    pub(super) fn to_mode(self, mode: LogicMode) -> Self {
        if self == Self::X && mode == LogicMode::TwoValue {
            return Self::L0;
        };
        self
    }

    /// Does `level` match this value?
    pub(super) fn matches(self, level: UdpLevelSymbol) -> bool {
        use UdpLevelSymbol as LS;
        match level {
            LS::L0 => self == Self::L0,
            LS::L1 => self == Self::L1,
            LS::X => self == Self::X,
            LS::QuestionMark => true,
            LS::B => self != Self::X,
        }
    }

    pub fn into_fv_logic_value(self) -> FvLogicValue {
        match self {
            Level::X => FvLogicValue::X,
            Level::L0 => FvLogicValue::L0,
            Level::L1 => FvLogicValue::L1,
        }
    }
}

impl From<UdpOutputSymbol> for Level {
    fn from(value: UdpOutputSymbol) -> Self {
        match value {
            UdpOutputSymbol::L0 => Level::L0,
            UdpOutputSymbol::L1 => Level::L1,
            UdpOutputSymbol::X => Level::X,
        }
    }
}

impl Transition {
    pub const ALL: Self = Self(0b111_111_111);
    pub const EMPTY: Self = Self(0b000_000_000);
    /// Every change: `*` and `(??)`.
    pub const QM_TO_QM: Self = Self(0b011_101_110);

    /// No change.
    pub const CONSTANT: Self = Self(0b100_010_001);
    pub const X: Self = Self(0b000_000_001);
    pub const L: Self = Self(0b000_010_000);
    pub const H: Self = Self(0b100_000_000);

    pub const X_TO_L: Self = Self(0b000_000_010); // (x0)
    pub const X_TO_H: Self = Self(0b000_000_100); // (x1)
    pub const L_TO_X: Self = Self(0b000_001_000); // (0x)
    pub const L_TO_H: Self = Self(0b000_100_000); // (01)
    pub const H_TO_X: Self = Self(0b001_000_000); // (1x)
    pub const H_TO_L: Self = Self(0b010_000_000); // (10)

    pub const TO_X: Self = Self(0b001_001_001);
    pub const TO_L: Self = Self(0b010_010_010);
    pub const TO_H: Self = Self(0b100_100_100);
    pub const TO_B: Self = Self(0b110_110_110);
    pub const TO_QM: Self = Self(0b111_111_111);

    /// `p`: `(01)`, `(0x)` and `(x1)`.
    pub const POSEDGE: Self = Self(Self::L_TO_H.0 | Self::L_TO_X.0 | Self::X_TO_H.0);
    /// `n`: `(10)`, `(1x)` and `(x0)`.
    pub const NEGEDGE: Self = Self(Self::H_TO_L.0 | Self::H_TO_X.0 | Self::X_TO_L.0);
    pub const RISING: Self = Self::L_TO_H;
    pub const FALLING: Self = Self::H_TO_L;

    /// The set with only the transition from `before` to `after`.
    pub fn new(before: Level, after: Level) -> Self {
        Self(1 << (3 * (before as u32) + (after as u32)))
    }

    /// The constant that has `value` stay `value`.
    pub fn constant(value: Level) -> Self {
        Self(1 << (3 * (value as u32) + (value as u32)))
    }

    pub fn set(&mut self, value: Self, condition: bool) {
        if condition {
            *self |= value;
        } else {
            *self &= !value;
        }
    }

    pub fn contains(self, value: Self) -> bool {
        self & value == value
    }

    pub fn from_end_level(level: UdpLevelSymbol) -> Self {
        use UdpLevelSymbol as LS;
        match level {
            LS::L0 => Self::TO_L,
            LS::L1 => Self::TO_H,
            LS::X => Self::TO_X,
            LS::QuestionMark => Self::TO_QM,
            LS::B => Self::TO_B,
        }
    }

    /// Does this match an input that did not change? This holds for every level and for no edge.
    pub fn is_level(self) -> bool {
        self & Self::CONSTANT != Self::EMPTY
    }

    /// Does this depend on the value before: is it an edge?
    pub fn is_edge(self) -> bool {
        !self.is_level()
    }

    /// The values before of the transitions in this set, by the discriminant of their [`Level`].
    pub fn before_values(self) -> BitsetU8 {
        (0..Level::ALL.len())
            .filter(|&before| self.0 >> (3 * before) & 0b111 != 0)
            .collect()
    }

    /// The values after of the transitions in this set, by the discriminant of their [`Level`]. For
    /// a level, these are the values it matches.
    pub fn after_values(self) -> BitsetU8 {
        (0..Level::ALL.len())
            .filter(|&after| self.0 & Self::TO_X.0 << after != 0)
            .collect()
    }
}

impl From<UdpEdgeIndicator> for Transition {
    fn from(value: UdpEdgeIndicator) -> Self {
        use UdpEdgeIndicator as I;
        use UdpEdgeSymbol as ES;
        use UdpLevelSymbol as LS;
        match value {
            I::Levels(before, after) => {
                fn level_to_mask(l: LS) -> u8 {
                    match l {
                        LS::L0 => 0b010,
                        LS::L1 => 0b100,
                        LS::X => 0b001,
                        LS::QuestionMark => 0b111,
                        LS::B => 0b110,
                    }
                }

                let before = level_to_mask(before.item);
                let after = level_to_mask(after.item);

                let mut res = Self::EMPTY;
                res.set(Self::X_TO_L, (before & 0b001 != 0) & (after & 0b010 != 0));
                res.set(Self::X_TO_H, (before & 0b001 != 0) & (after & 0b100 != 0));
                res.set(Self::L_TO_X, (before & 0b010 != 0) & (after & 0b001 != 0));
                res.set(Self::L_TO_H, (before & 0b010 != 0) & (after & 0b100 != 0));
                res.set(Self::H_TO_X, (before & 0b100 != 0) & (after & 0b001 != 0));
                res.set(Self::H_TO_L, (before & 0b100 != 0) & (after & 0b010 != 0));
                res
            }
            I::Edge(edge) => match edge.item {
                ES::R => Self::RISING,
                ES::F => Self::FALLING,
                ES::P => Self::POSEDGE,
                ES::N => Self::NEGEDGE,
                ES::Star => Self::QM_TO_QM,
            },
        }
    }
}

impl BitOr for Transition {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitAnd for Transition {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitXor for Transition {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitOrAssign for Transition {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}
impl BitAndAssign for Transition {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}
impl BitXorAssign for Transition {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}
impl Not for Transition {
    type Output = Self;
    fn not(self) -> Self::Output {
        self ^ Self::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstItem;

    const VALUES: [Level; 3] = [Level::X, Level::L0, Level::L1];

    /// The set of the transitions for which `f` is true.
    fn set_of(f: impl Fn(Level, Level) -> bool) -> Transition {
        let mut res = Transition::EMPTY;
        for b in VALUES {
            for a in VALUES {
                res.set(Transition::new(b, a), f(b, a));
            }
        }
        res
    }

    /// The set of the values for which `f` is true, by their discriminant.
    fn values_of(f: impl Fn(Level) -> bool) -> BitsetU8 {
        VALUES
            .into_iter()
            .filter(|&v| f(v))
            .map(|v| v as usize)
            .collect()
    }

    #[test]
    fn constants_are_their_transitions() {
        use Level::*;
        assert_eq!(Transition::CONSTANT, set_of(|b, a| b == a));
        assert_eq!(Transition::QM_TO_QM, set_of(|b, a| b != a));
        assert_eq!(Transition::X, Transition::new(X, X));
        assert_eq!(Transition::L, Transition::new(L0, L0));
        assert_eq!(Transition::H, Transition::new(L1, L1));
        assert_eq!(Transition::X_TO_L, Transition::new(X, L0));
        assert_eq!(Transition::X_TO_H, Transition::new(X, L1));
        assert_eq!(Transition::L_TO_X, Transition::new(L0, X));
        assert_eq!(Transition::L_TO_H, Transition::new(L0, L1));
        assert_eq!(Transition::H_TO_X, Transition::new(L1, X));
        assert_eq!(Transition::H_TO_L, Transition::new(L1, L0));
        assert_eq!(
            Transition::POSEDGE,
            set_of(|b, a| matches!((b, a), (L0, L1) | (L0, X) | (X, L1)))
        );
        assert_eq!(
            Transition::NEGEDGE,
            set_of(|b, a| matches!((b, a), (L1, L0) | (L1, X) | (X, L0)))
        );
    }

    #[test]
    fn levels_only_constrain_the_value_after() {
        use UdpLevelSymbol as LS;
        for level in [LS::L0, LS::L1, LS::X, LS::QuestionMark, LS::B] {
            let set = Transition::from_end_level(level);
            assert_eq!(set, set_of(|_, a| a.matches(level)));
            assert_eq!(set.after_values(), values_of(|v| v.matches(level)));
            assert_eq!(set.before_values(), values_of(|_| true));
            assert!(set.is_level());
            assert!(!set.is_edge());
        }
    }

    #[test]
    fn edge_symbols_require_a_change() {
        use UdpEdgeSymbol as ES;
        for symbol in [ES::R, ES::F, ES::P, ES::N, ES::Star] {
            let set = Transition::from(UdpEdgeIndicator::Edge(AstItem {
                item: symbol,
                loc: 0,
            }));
            assert!(!set.is_level());
            assert!(set.is_edge());
        }
    }

    #[test]
    fn edge_levels_require_a_change() {
        use UdpLevelSymbol as LS;
        let levels = [LS::L0, LS::L1, LS::X, LS::QuestionMark, LS::B];
        for before in levels {
            for after in levels {
                let item = |l| AstItem { item: l, loc: 0 };
                let set = Transition::from(UdpEdgeIndicator::Levels(item(before), item(after)));
                assert_eq!(
                    set,
                    set_of(|b, a| b != a && b.matches(before) && a.matches(after))
                );
                assert!(!set.is_level());
                assert!(set.is_edge());
            }
        }
    }

    #[test]
    fn before_and_after_values() {
        use Level::*;
        assert_eq!(
            Transition::POSEDGE.before_values(),
            values_of(|v| matches!(v, X | L0))
        );
        assert_eq!(
            Transition::POSEDGE.after_values(),
            values_of(|v| matches!(v, X | L1))
        );
        assert_eq!(Transition::RISING.before_values(), values_of(|v| v == L0));
        assert_eq!(Transition::RISING.after_values(), values_of(|v| v == L1));
        assert_eq!(Transition::EMPTY.before_values(), values_of(|_| false));
        assert_eq!(Transition::ALL.after_values(), values_of(|_| true));
    }
}
