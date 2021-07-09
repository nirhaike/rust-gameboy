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
	high_data: [u8; range_size!(MMAP_RAM_HIGH)],
}

impl InternalRam {
	/// Initialize the internal ram.
	pub fn new() -> Self {
		InternalRam {
			data: [0_u8; range_size!(MMAP_RAM_INTERNAL)],
			high_data: [0_u8; range_size!(MMAP_RAM_HIGH)],
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
				panic!();
			}
		}
	}

	/// Returns the mapped offset within the high ram for the given address.
	fn hram_offset(&self, address: u16) -> usize {
		match address {
			memory_range!(MMAP_RAM_HIGH) => {
				(address as usize - range_start!(MMAP_RAM_HIGH)) as usize
			}
			_ => {
				panic!();
			}
		}
	}
}

impl Memory for InternalRam {
	/// Write to the internal ram.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		match address {
			memory_range!(MMAP_RAM_INTERNAL) |
			memory_range!(MMAP_RAM_ECHO) => {
				self.data[self.offset(address)] = value;
				Ok(())
			}
			memory_range!(MMAP_RAM_HIGH) => {
				self.high_data[self.hram_offset(address)] = value;
				Ok(())
			}
			_ => {
				Err(GameboyError::Io("ram_write: Attempt to write out of bounds."))
			}
		}
	}

	/// Read from the internal ram.
	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		match address {
			memory_range!(MMAP_RAM_INTERNAL) |
			memory_range!(MMAP_RAM_ECHO) => {
				Ok(self.data[self.offset(address)])
			}
			memory_range!(MMAP_RAM_HIGH) => {
				Ok(self.high_data[self.hram_offset(address)])
			}
			_ => {
				Err(GameboyError::Io("ram_read: Attempt to read out of bounds."))
			}
		}
	}
}
