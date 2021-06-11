// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulate the gameboy's memory mapping and bus access.

#[macro_use]
pub mod memory_range;
pub mod cartridge;
pub mod rtc;
pub mod ram;
pub mod io;

use io::*;
use ram::*;
use cartridge::*;
use memory_range::*;

use crate::config::Config;
use crate::GameboyError;

/// Bus locations-related constants.
#[allow(missing_docs)]
pub mod consts {
	use super::*;

	pub const MMAP_ROM_BANK0: MemoryRange = make_range!(0x0000, 0x3FFF);
	/// Switchable ROM bank.
	pub const MMAP_ROM_BANK_SW: MemoryRange = make_range!(0x4000, 0x7FFF);
	pub const MMAP_VIDEO_RAM: MemoryRange = make_range!(0x8000, 0x9FFF);
	/// Switchable RAM bank.
	pub const MMAP_RAM_BANK_SW: MemoryRange = make_range!(0xA000, 0xBFFF);
	pub const MMAP_RAM_INTERNAL: MemoryRange = make_range!(0xC000, 0xDFFF);
	/// Maps to the same physical memory as the internal ram.
	pub const MMAP_RAM_ECHO: MemoryRange = make_range!(0xE000, 0xFDFF);
	/// Sprite/Object attribute memory.
	pub const MMAP_SPRITE_OAM: MemoryRange = make_range!(0xFE00, 0xFE9F);
	pub const MMAP_IO_PORTS: MemoryRange = make_range!(0xFF00, 0xFF4B);
	/// High RAM.
	pub const MMAP_RAM_HIGH: MemoryRange = make_range!(0xFF80, 0xFFFE);
	/// Interrupt enable register.
	pub const MMAP_INTERRUPT_EN: MemoryRange = make_range!(0xFFFF, 0xFFFF);
}

use consts::*;

/// A peripheral that can be written and read by the cpu.
pub trait Memory {
	/// Write a 8-bit value to the peripheral.
	///
	/// * `address` - The absolute memory address to write into.
	/// * `value` - The value to write.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError>;

	/// Read a 8-bit value from this peripheral.
	///
	/// * `address` - The absolute memory address to read from.
	fn read(&self, address: u16) -> Result<u8, GameboyError>;
}

/// A virtual representation of Gameboy (Color) memory bus.
///
/// This implementation provides memory/peripheral abstraction.
pub struct SystemBus<'a> {
	pub(crate) cartridge: &'a mut Cartridge<'a>,
	pub(crate) io: IOPorts,
	pub(crate) ram: InternalRam,
}

/// An abstraction for fetching mutable and immutable regions.
macro_rules! get_region {
	($name:tt $(,$mut_:tt)*) => {
		/// Returns the region that contains the given address.
		fn $name(&$($mut_)* self, address: u16) -> Result<&$($mut_)* dyn Memory, GameboyError> {
			match address {
				// Cartridge-mapped offsets
				memory_range!(MMAP_ROM_BANK0) |
				memory_range!(MMAP_ROM_BANK_SW) |
				memory_range!(MMAP_RAM_BANK_SW) => {
					Ok(&$($mut_)* (*self.cartridge))
				}
				// Internal RAM
				memory_range!(MMAP_RAM_INTERNAL) |
				memory_range!(MMAP_RAM_ECHO) => {
					Ok(&$($mut_)* self.ram)
				}
				// I/O registers
				memory_range!(MMAP_IO_PORTS) |
				memory_range!(MMAP_INTERRUPT_EN) => {
					Ok(&$($mut_)* self.io)
				}
				_ => {
					Err(GameboyError::Io("Accessed an unmapped region."))
				}
			}
		}
	}
}

impl<'a> SystemBus<'a> {
	/// Initialize a new address space.
	pub fn new(config: &'a Config, cartridge: &'a mut Cartridge<'a>) -> Self {
		SystemBus {
			cartridge,
			io: IOPorts::new(config),
			ram: InternalRam::new(),
		}
	}

	// Get an immutable region
	get_region!(region);

	// Get a mutable region
	get_region!(region_mut, mut);
}

impl<'a> Memory for SystemBus<'a> {
	/// Handle reading from a memory region.
	/// The function calls the relevent peripheral's implementation.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		let peripheral = self.region_mut(address)?;

		peripheral.write(address, value)
	}

	/// Handle writing to a memory region.
	/// The function calls the relevent peripheral's implementation.
	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		let peripheral = self.region(address)?;
		
		peripheral.read(address)
	}
}

#[cfg(test)]
impl<'a> SystemBus<'a> {
	/// Writes the complete array's bytes to the relevant memory region.
	pub fn write_all(&mut self, address: u16, array: &[u8]) -> Result<(), GameboyError> {
		for (index, value) in array.iter().enumerate() {
			self.write(address + (index as u16), *value)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_range() {
    	let int_enable_ptr: u16 = 0xFFFF;
    	let ram_ptr: u16 = 0xA100;

    	match int_enable_ptr {
    		memory_range!(MMAP_INTERRUPT_EN) => { }
    		_ => { assert!(false); }
    	}

    	match ram_ptr {
    		memory_range!(MMAP_RAM_BANK_SW) => { }
    		_ => { assert!(false); }
    	}
    }
}
