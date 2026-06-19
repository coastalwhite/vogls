use crate::encoding::XRegIdent;
use crate::error::{AssembleError, AssembleResult};

pub struct SourceCursor<'a> {
    content: &'a str,
    pub(crate) offset: usize,
    line: usize,
    line_offset: usize,
}

impl<'a> SourceCursor<'a> {
    pub const fn new(content: &'a str) -> Self {
        Self {
            content,
            offset: 0,
            line: 0,
            line_offset: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.content.len() == self.offset
    }

    pub(crate) fn new_err(
        &self,
        reason: impl Into<std::borrow::Cow<'static, str>>,
    ) -> AssembleError {
        AssembleError {
            reason: reason.into(),
            line: self.line,
            line_offset: self.line_offset,
            _offset: self.offset,
        }
    }

    /// Trim whitespace and comments at the current position of the cursor.
    pub fn trim_wsc(&mut self) {
        let bs = self.content.as_bytes();
        while let Some(&b) = bs.get(self.offset) {
            // Handle whitespace
            if b.is_ascii_whitespace() {
                self.line += usize::from(b == b'\n');
                self.offset += 1;
                continue;
            }

            // Handle comments
            if b == b'#' {
                self.offset += 1;
                while let Some(&b) = bs.get(self.offset)
                    && b != b'\n'
                {
                    self.offset += 1;
                }

                if !self.is_empty() {
                    self.line += 1;
                    self.offset += 1;
                    self.line_offset = self.offset;
                }

                continue;
            }

            break;
        }
    }

    pub fn peek_byte(&self) -> Option<u8> {
        self.content.as_bytes().get(self.offset).copied()
    }
    pub fn next_byte(&mut self) -> Option<u8> {
        let b = self.content.as_bytes().get(self.offset).copied();
        self.offset += 1;
        b
    }
    pub fn expect_byte(&mut self, expected: u8) -> AssembleResult<()> {
        debug_assert_ne!(expected, b'\n');
        let Some(b) = self.peek_byte() else {
            return Err(Box::new(
                self.new_err(format!("expected '{expected}', got end-of-file.'")),
            ));
        };
        if b != expected {
            return Err(Box::new(
                self.new_err(format!("expected '{expected}', got '{b}'")),
            ));
        }
        self.offset += 1;
        Ok(())
    }
    pub fn take_ident(&mut self) -> AssembleResult<&'a str> {
        if !self.is_next_first_ident_char() {
            return Err(Box::new(self.new_err("expected identifier")));
        };

        let mut i = self.offset + 1;
        let bs = self.content.as_bytes();
        while let Some(&b) = bs.get(i)
            && matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'$' | b'_' | b'0'..=b'9')
        {
            i += 1;
        }
        let start = self.offset;
        self.offset = i;
        Ok(&self.content[start..i])
    }
    pub fn peek_ident(&self) -> Option<&'a str> {
        if !self.is_next_first_ident_char() {
            return None;
        }

        let mut i = self.offset + 1;
        let bs = self.content.as_bytes();
        while let Some(&b) = bs.get(i)
            && matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'$' | b'_' | b'0'..=b'9')
        {
            i += 1;
        }
        let start = self.offset;
        Some(&self.content[start..i])
    }
    pub fn is_next_first_ident_char(&self) -> bool {
        self.peek_byte()
            .is_some_and(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'$' | b'_'))
    }

    pub fn take_opt_separator(&mut self) {
        let bs = self.content.as_bytes();
        while let Some(&b) = bs.get(self.offset)
            && matches!(b, b' ' | b'\t')
        {
            self.offset += 1;
        }
    }

    pub fn take_separator(&mut self) -> AssembleResult<()> {
        let mut i = self.offset;
        let bs = self.content.as_bytes();
        while let Some(&b) = bs.get(i)
            && matches!(b, b' ' | b'\t')
        {
            i += 1;
        }
        if i == self.offset {
            return Err(Box::new(self.new_err("missing separator")));
        }
        self.offset = i;
        Ok(())
    }

    pub fn take_comma_separator(&mut self) -> AssembleResult<()> {
        self.take_opt_separator();
        if self.peek_byte() != Some(b',') {
            return Err(Box::new(self.new_err("missing comma")));
        }
        self.offset += 1;
        self.take_opt_separator();
        Ok(())
    }

    pub fn take_gp_register(&mut self) -> AssembleResult<XRegIdent> {
        let s = self.take_ident().map_err(|mut err| {
            err.reason = "expected register".into();
            err
        })?;
        let s = parse_gp_register(s).ok_or_else(|| Box::new(self.new_err("unknown register")))?;
        Ok(s)
    }

    pub fn take_stmt_end(&mut self) -> AssembleResult<()> {
        let start_line = self.line;
        self.trim_wsc();
        if self.line > start_line {
            return Ok(());
        }
        match self.peek_byte() {
            None => {}
            Some(b';') => self.offset += 1,
            Some(_) => return Err(Box::new(self.new_err("expected end of statement"))),
        }
        Ok(())
    }

    pub fn take_signed_imm32(&mut self) -> AssembleResult<i64> {
        // @TODO: Many other syntaxes.
        self.take_u32().map(|v| v as i64)
    }
    pub fn take_imm32(&mut self) -> AssembleResult<u32> {
        // @TODO: Many other syntaxes.
        self.take_u32()
    }
    pub fn take_imm12(&mut self) -> AssembleResult<i16> {
        // @TODO: Many other syntaxes.
        self.take_u32().map(|v| v as i16)
    }
    pub fn take_imm20(&mut self) -> AssembleResult<u32> {
        // @TODO: Many other syntaxes.
        self.take_u32().map(|v| v as u32)
    }
    pub fn take_imm12_unsigned(&mut self) -> AssembleResult<u32> {
        todo!()
    }
    pub fn take_shamt(&mut self) -> AssembleResult<u8> {
        // @TODO: Many other syntaxes.
        self.take_u32().map(|v| v as u8)
    }

    pub fn take_u32(&mut self) -> AssembleResult<u32> {
        let mut i = self.offset;
        let bs = self.content.as_bytes();
        let mut value = 0u32;
        while let Some(b) = bs.get(i)
            && b.is_ascii_digit()
        {
            value = value
                .checked_mul(10)
                .ok_or_else(|| self.new_err("overflow"))?;
            value = value
                .checked_add((b - b'0').into())
                .ok_or_else(|| self.new_err("overflow"))?;
            i += 1;
        }
        self.offset = i;
        Ok(value)
    }

    pub fn next_byte_if_equals(&mut self, byte: u8) -> bool {
        let equals = self.peek_byte() == Some(byte);
        self.offset += usize::from(equals);
        equals
    }
}

pub fn parse_gp_register(s: &str) -> Option<XRegIdent> {
    Some(match s {
        "zero" | "x0" => XRegIdent::Zero,
        "ra" | "x1" => XRegIdent::Ra,
        "sp" | "x2" => XRegIdent::Sp,
        "gp" | "x3" => XRegIdent::Gp,
        "tp" | "x4" => XRegIdent::Tp,
        "t0" | "x5" => XRegIdent::T0,
        "t1" | "x6" => XRegIdent::T1,
        "t2" | "x7" => XRegIdent::T2,
        "fp" | "s0" | "x8" => XRegIdent::Fp,
        "s1" | "x9" => XRegIdent::S1,
        "a0" | "x10" => XRegIdent::A0,
        "a1" | "x11" => XRegIdent::A1,
        "a2" | "x12" => XRegIdent::A2,
        "a3" | "x13" => XRegIdent::A3,
        "a4" | "x14" => XRegIdent::A4,
        "a5" | "x15" => XRegIdent::A5,
        "a6" | "x16" => XRegIdent::A6,
        "a7" | "x17" => XRegIdent::A7,
        "s2" | "x18" => XRegIdent::S2,
        "s3" | "x19" => XRegIdent::S3,
        "s4" | "x20" => XRegIdent::S4,
        "s5" | "x21" => XRegIdent::S5,
        "s6" | "x22" => XRegIdent::S6,
        "s7" | "x23" => XRegIdent::S7,
        "s8" | "x24" => XRegIdent::S8,
        "s9" | "x25" => XRegIdent::S9,
        "s10" | "x26" => XRegIdent::S10,
        "s11" | "x27" => XRegIdent::S11,
        "t3" | "x28" => XRegIdent::T3,
        "t4" | "x29" => XRegIdent::T4,
        "t5" | "x30" => XRegIdent::T5,
        "t6" | "x31" => XRegIdent::T6,
        _ => return None,
    })
}
