use std::fmt;

use crate::arithmetic::FvLogicValue;
use crate::{Bits, VectorSize};

impl fmt::Binary for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn print(
            f: &mut impl fmt::Write,
            val: u64,
            spc: Option<u64>,
            size: VectorSize,
        ) -> fmt::Result {
            assert!(size.get() <= 64);
            match spc {
                None => write!(f, "{val:064b}"),
                Some(spc) => {
                    for i in 0..size.get() {
                        let shift = size.get() - i - 1;
                        let s = (spc >> shift) & 1;
                        let v = (val >> shift) & 1;
                        let fv = FvLogicValue::from_repr(((s as u8) << 1) | (v as u8));

                        f.write_char(match fv {
                            FvLogicValue::X => 'x',
                            FvLogicValue::Z => 'z',
                            FvLogicValue::L0 => '0',
                            FvLogicValue::L1 => '1',
                        })?;
                    }
                    Ok(())
                }
            }
        }

        let data_ref = self.as_data_ref();
        let (val, spc) = data_ref.to_u64_slices();
        let msw_val = *val.last().unwrap();
        let msw_size = self.size().get() % 64;
        let rem_size = self.size().get() - msw_size;
        match spc {
            None => {
                if let Some(msw_size) = VectorSize::new(msw_size) {
                    print(f, msw_val, None, msw_size)?;
                }
                for i in (0..rem_size.div_ceil(64) as usize).rev() {
                    print(f, val[i], None, VectorSize::new(64).unwrap())?;
                }
            }
            Some(spc) => {
                if let Some(msw_size) = VectorSize::new(msw_size) {
                    let msw_spc = *spc.last().unwrap();
                    print(f, msw_val, Some(msw_spc), msw_size)?;
                }
                for i in (0..rem_size.div_ceil(64) as usize).rev() {
                    print(f, val[i], Some(spc[i]), VectorSize::new(64).unwrap())?;
                }
            }
        }

        Ok(())
    }
}

fn format_hex(bits: &Bits, f: &mut fmt::Formatter<'_>, is_lower: bool) -> fmt::Result {
    let size = bits.size();
    write!(f, "{size}'h")?;
    fn print(
        f: &mut impl fmt::Write,
        val: u64,
        spc: Option<u64>,
        size: VectorSize,
        is_lower: bool,
    ) -> fmt::Result {
        assert!(size.get() > 0 && size.get() <= 64);
        match spc {
            None => write!(
                f,
                "{val:0ndigits$x}",
                ndigits = size.get().div_ceil(4) as usize
            ),
            Some(spc) => {
                let mut rem_size = size.get();
                while rem_size > 0 {
                    let shift = if rem_size % 4 == 0 {
                        rem_size - 4
                    } else {
                        rem_size - rem_size % 4
                    };
                    let s = (spc >> shift) & 0xF;
                    let v = (val >> shift) & 0xF;

                    let mask = (1u64 << (rem_size - shift)) - 1;
                    let c = if s == mask {
                        if v >= 10 {
                            if is_lower {
                                b'a' + (v as u8) - 10
                            } else {
                                b'A' + (v as u8) - 10
                            }
                        } else {
                            b'0' + v as u8
                        }
                        .into()
                    } else if s == 0 && v == 0 {
                        'x'
                    } else if s == 0 && v == mask {
                        'z'
                    } else if !s & !v != 0 {
                        'X'
                    } else {
                        'Z'
                    };
                    f.write_char(c)?;

                    rem_size = shift;
                }
                Ok(())
            }
        }
    }

    let data_ref = bits.as_data_ref();
    let (val, spc) = data_ref.to_u64_slices();
    let msw_val = *val.last().unwrap();
    let msw_size = size.get() % 64;
    let rem_size = size.get() - msw_size;
    match spc {
        None => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                print(f, msw_val, None, msw_size, is_lower)?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(f, val[i], None, VectorSize::new(64).unwrap(), is_lower)?;
            }
        }
        Some(spc) => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                let msw_spc = *spc.last().unwrap();
                print(f, msw_val, Some(msw_spc), msw_size, is_lower)?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(
                    f,
                    val[i],
                    Some(spc[i]),
                    VectorSize::new(64).unwrap(),
                    is_lower,
                )?;
            }
        }
    }

    Ok(())
}

impl fmt::LowerHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_hex(self, f, true)
    }
}
impl fmt::UpperHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_hex(self, f, false)
    }
}
