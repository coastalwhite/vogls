pub trait FileIdFfi {
    fn from_ffi(value: u64) -> Self;
    fn to_ffi(self) -> u64;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u64);

impl FileIdFfi for FileId {
    fn from_ffi(value: u64) -> Self {
        Self(value)
    }
    fn to_ffi(self) -> u64 {
        self.0
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct FileOpenOptions(u64);

impl FileOpenOptions {
    pub const EMPTY: Self = Self(0b000_0000u64);

    pub const READ: Self = Self(0b000_0001u64);
    pub const WRITE: Self = Self(0b000_0010u64);
    pub const CREATE: Self = Self(0b000_0100u64);
    pub const CREATE_NEW: Self = Self(0b000_1000u64);
    pub const APPEND: Self = Self(0b001_0000u64);
    pub const TRUNCATE: Self = Self(0b010_0000u64);
    pub const INCLUDE_DIRS: Self = Self(0b100_0000u64);

    pub const fn new() -> Self {
        Self::EMPTY
    }

    pub const fn read(&mut self, read: bool) -> &mut Self {
        self.set(Self::READ, read);
        self
    }
    pub const fn write(&mut self, write: bool) -> &mut Self {
        self.set(Self::WRITE, write);
        self
    }
    pub const fn create(&mut self, create: bool) -> &mut Self {
        self.set(Self::CREATE, create);
        self
    }
    pub const fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.set(Self::CREATE_NEW, create_new);
        self
    }
    pub const fn append(&mut self, append: bool) -> &mut Self {
        self.set(Self::APPEND, append);
        self
    }
    pub const fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.set(Self::TRUNCATE, truncate);
        self
    }
    pub const fn include_dirs(&mut self, include_dirs: bool) -> &mut Self {
        self.set(Self::INCLUDE_DIRS, include_dirs);
        self
    }

    pub const fn set(&mut self, v: Self, set: bool) {
        if set {
            self.0 |= v.0;
        } else {
            self.0 &= !v.0;
        }
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
