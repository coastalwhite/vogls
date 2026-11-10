#[derive(Debug, Clone)]
pub struct ElfStringTable {
    content: Vec<u8>,
}

impl ElfStringTable {
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn contains_section_headers(&self) -> bool {
        let mut prev_was_zero = false;

        for b in self.content.iter() {
            if prev_was_zero && *b != b'.' {
                return false;
            }

            prev_was_zero = *b == b'\0';
        }

        true
    }

    /// A table without any strings, for a file that has none.
    pub fn empty() -> ElfStringTable {
        ElfStringTable {
            content: Vec::new(),
        }
    }

    pub fn new(bytes: &[u8], size: usize) -> ElfStringTable {
        ElfStringTable {
            content: bytes[..size].to_vec(),
        }
    }

    /// The string at `start`, up to the next NUL byte. Empty when `start`
    /// lies past the end of the table.
    pub fn get_from(&self, start: usize) -> &[u8] {
        let Some(rest) = self.content.get(start..) else {
            return &[];
        };
        let end = rest.iter().position(|&b| b == b'\0').unwrap_or(rest.len());
        &rest[..end]
    }

    pub fn entries<'a>(&'a self) -> ElfStringTableEntries<'a> {
        ElfStringTableEntries {
            string_table: self,
            offset: 0,
        }
    }
}

pub struct ElfStringTableEntries<'a> {
    string_table: &'a ElfStringTable,
    offset: usize,
}

impl ElfStringTableEntries<'_> {
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl<'a> Iterator for ElfStringTableEntries<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let content = self.string_table.content();

        // Find next b'\0'
        while self.offset != content.len() && content[self.offset] != b'\0' {
            self.offset += 1;
        }

        // Check whether this is the closing byte
        if self.offset == content.len() - 1 {
            return None;
        }

        // Move past b'\0'
        self.offset += 1;

        // Find the end of the entry
        let mut end = self.offset;
        while (end) != content.len() && content[end] != b'\0' {
            end += 1;
        }

        Some(&content[self.offset..end])
    }
}
