// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Emulate the gameboy's intermal RAM.

use super::Memory;
use super::consts::*;
use super::memory_range::*;

use crate::GameboyError;

/// Gameboy's internal memory.
pub struct InternalRam {
	data: [u8; range_size!(MMAP_RAM_INTERNAL)],
}

impl InternalRam {
	/// Initialize the internal ram.
	pub fn new() -> Self {
		InternalRam {
			data: [0_u8; range_size!(MMAP_RAM_INTERNAL)],
		}
	}

	/// Returns the mapped offset within the ram for the given address.
	///
	/// The ram has two memory ranges mapped to it (MMAP_RAM_INTERNAL and MMAP_RAM_ECHO).
	/// This function resolves the current range and returns the offset relative to it.
	fn offset(&self, address: u16) -> usize {
		match address {
			memory_range!(MMAP_RAM_INTERNAL) => {
				(address as usize - range_start!(MMAP_RAM_INTERNAL)) as usize
			}
			memory_range!(MMAP_RAM_ECHO) => {
				(address as usize - range_start!(MMAP_RAM_ECHO)) as usize
			}
			_ => {
				unimplemented!();
			}
		}
	}
}

impl Memory for InternalRam {
	/// Write to the internal ram.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		let offset = self.offset(address);

		if offset >= self.data.len() {
			return Err(GameboyError::Io("ram_write: Attempt to write out of bounds."));
		}

		self.data[offset] = value;

		Ok(())
	}

	/// Read from the internal ram.
	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		let offset = self.offset(address);

		if offset >= self.data.len() {
			return Err(GameboyError::Io("ram_read: Attempt to read out of bounds."));
		}

		Ok(self.data[offset])
	}
}
