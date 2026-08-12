use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeUnit {
    Femtoseconds,
    Picoseconds,
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimeSize {
    N1,
    N10,
    N100,
}
impl TimeSize {
    pub fn into_u64(self) -> u64 {
        match self {
            TimeSize::N1 => 1,
            TimeSize::N10 => 10,
            TimeSize::N100 => 100,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            TimeSize::N1 => "1",
            TimeSize::N10 => "10",
            TimeSize::N100 => "100",
        }
    }
}
impl TimeUnit {
    pub fn convert_from_fs(self, fs: u64) -> u64 {
        match self {
            TimeUnit::Seconds => fs * 10u64.pow(15),
            TimeUnit::Milliseconds => fs * 10u64.pow(12),
            TimeUnit::Microseconds => fs * 10u64.pow(9),
            TimeUnit::Nanoseconds => fs * 10u64.pow(6),
            TimeUnit::Picoseconds => fs * 10u64.pow(3),
            TimeUnit::Femtoseconds => fs,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            TimeUnit::Seconds => "s",
            TimeUnit::Milliseconds => "ms",
            TimeUnit::Microseconds => "us",
            TimeUnit::Nanoseconds => "ns",
            TimeUnit::Picoseconds => "ps",
            TimeUnit::Femtoseconds => "fs",
        }
    }
}

impl FromStr for TimeSize {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::N1),
            "10" => Ok(Self::N10),
            "100" => Ok(Self::N100),
            _ => Err(()),
        }
    }
}

impl FromStr for TimeUnit {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "s" => Ok(Self::Seconds),
            "ms" => Ok(Self::Milliseconds),
            "us" => Ok(Self::Microseconds),
            "ns" => Ok(Self::Nanoseconds),
            "ps" => Ok(Self::Picoseconds),
            "fs" => Ok(Self::Femtoseconds),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeResolution {
    pub unit: TimeUnit,
    pub size: TimeSize,
}

impl TimeResolution {
    pub const S1: Self = Self {
        unit: TimeUnit::Seconds,
        size: TimeSize::N1,
    };
    pub const NS1: Self = Self {
        unit: TimeUnit::Nanoseconds,
        size: TimeSize::N1,
    };

    pub const fn pow10_over_fs(self) -> u32 {
        (self.unit as u32 * 3) + (self.size as u32)
    }

    pub const fn truncate_or_multiply_to(self, value: u64, to: TimeResolution) -> u64 {
        let from_p10 = self.pow10_over_fs();
        let to_p10 = to.pow10_over_fs();

        // @Performance: Use lookup table instead of pow.
        if from_p10 == to_p10 {
            value
        } else if from_p10 > to_p10 {
            // Multiplication
            value * 10u64.pow(from_p10 - to_p10)
        } else {
            // Truncate
            value / 10u64.pow(to_p10 - from_p10)
        }
    }

    pub fn truncate_or_multiply_f64_to(self, value: f64, to: TimeResolution) -> f64 {
        let from_p10 = self.pow10_over_fs();
        let to_p10 = to.pow10_over_fs();

        // @Performance: Use lookup table instead of pow.
        if from_p10 == to_p10 {
            value
        } else if from_p10 > to_p10 {
            // Multiplication
            value * 10f64.powi((from_p10 - to_p10) as i32)
        } else {
            // Truncate
            value / 10f64.powi((to_p10 - from_p10) as i32)
        }
    }
}
