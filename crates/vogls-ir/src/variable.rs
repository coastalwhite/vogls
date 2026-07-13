use std::fmt;
use std::num::NonZeroU64;

use vogls_bits::VectorSize;
use vogls_utils::VgHashMap;

use crate::{LogicMode, SCALAR_VSIZE};

/// A unique identifier for a VIR variable.
///
/// This is a unique identifier combined with a conditionally inlined size. If the size does not
/// fit in the allocated space, it put into the external [`VariableMap`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VariableKey(NonZeroU64);

/// Manages the allocated variables.
#[derive(Default, Clone)]
pub struct VariableMap {
    prev_var_identifier: u64,
    non_inlined_var_sizes: VgHashMap<u64, VectorSize>,
}

impl VariableKey {
    const MAX_INLINE_SIZE: VectorSize = VectorSize::new(255).unwrap();

    fn inlined_size(self) -> Option<VectorSize> {
        VectorSize::new((self.0.get() & 0xFF) as u32)
    }
    pub fn mode(self) -> LogicMode {
        if (self.0.get() & (1u64 << 8)) != 0 {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        }
    }

    pub fn identifier(self) -> u64 {
        self.0.get() >> 9
    }

    fn from_id_and_size(id: u64, mode: LogicMode, size: VectorSize) -> Self {
        let capped_size = Some(size.get())
            .filter(|v| *v <= Self::MAX_INLINE_SIZE.get())
            .unwrap_or(0) as u64;
        let value = (id << 9) | ((mode as u64) << 8) | capped_size;
        let value = NonZeroU64::new(value).expect("should never be zero");
        VariableKey(value)
    }

    fn size(self, non_inlined_var_sizes: &VgHashMap<u64, VectorSize>) -> VectorSize {
        match self.inlined_size() {
            None => non_inlined_var_sizes[&self.identifier()],
            Some(size) => size,
        }
    }

    fn update(
        &mut self,
        new_mode: LogicMode,
        new_size: VectorSize,
        non_inlined_var_sizes: &mut VgHashMap<u64, VectorSize>,
    ) {
        non_inlined_var_sizes.remove(&self.identifier());
        *self = Self::from_id_and_size(self.identifier(), new_mode, new_size);
        if new_size > Self::MAX_INLINE_SIZE {
            non_inlined_var_sizes.insert(self.identifier(), new_size);
        }
    }

    #[inline(always)]
    pub fn is_scalar(self) -> bool {
        self.inlined_size() == Some(SCALAR_VSIZE)
    }
}

impl fmt::Debug for VariableKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableKey")
            .field("id", &self.identifier())
            .field("mode", &self.mode())
            .field("inline_size", &self.inlined_size())
            .finish()
    }
}

impl VariableMap {
    pub fn size(&self, key: VariableKey) -> VectorSize {
        key.size(&self.non_inlined_var_sizes)
    }

    pub fn insert(&mut self, mode: LogicMode, size: VectorSize) -> VariableKey {
        self.prev_var_identifier += 1;
        let key = VariableKey::from_id_and_size(self.prev_var_identifier, mode, size);
        if size.get() > 255 {
            self.non_inlined_var_sizes.insert(key.identifier(), size);
        }
        key
    }

    pub fn remove(&mut self, key: VariableKey) {
        if key.inlined_size().is_none() {
            self.non_inlined_var_sizes.remove(&key.identifier());
        }
    }

    pub fn update(&mut self, key: &mut VariableKey, new_mode: LogicMode, new_size: VectorSize) {
        key.update(new_mode, new_size, &mut self.non_inlined_var_sizes);
    }
}
