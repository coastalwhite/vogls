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

    pub fn new(bytes: &[u8], size: usize) -> ElfStringTable {
        ElfStringTable {
            content: bytes[..size].to_vec(),
        }
    }

    pub fn get_from(&self, start: usize) -> &[u8] {
        for (i, b) in self.content.iter().enumerate().skip(start) {
            if *b == b'\0' {
                return &self.content[start..i];
            }
        }

        &self.content[start..self.content.len()]
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
