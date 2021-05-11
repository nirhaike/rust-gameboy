// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulate the gameboy's memory mapping and bus access.

use crate::cartridge::*;
use crate::GameboyError;

/// Bus locations-related constants.
#[allow(missing_docs)]
pub mod consts {

	pub type MemoryRange = core::ops::RangeInclusive<u16>;

	pub const MMAP_ROM_BANK0: MemoryRange = 0x0000..=0x3FFF;
	/// Switchable ROM bank.
	pub const MMAP_ROM_BANK_SW: MemoryRange = 0x4000..=0x7FFF;
	pub const MMAP_VIDEO_RAM: MemoryRange = 0x8000..=0x9FFF;
	/// Switchable RAM bank.
	pub const MMAP_RAM_BANK_SW: MemoryRange = 0xA000..=0xBFFF;
	pub const MMAP_RAM_INTERNAL: MemoryRange = 0xC000..=0xDFFF;
	/// Maps to the same physical memory as the internal ram.
	pub const MMAP_RAM_ECHO: MemoryRange = 0xE000..=0xFDFF;
	/// Sprite/Object attribute memory.
	pub const MMAP_SPRITE_OAM: MemoryRange = 0xFE00..=0xFE9F;
	pub const MMAP_IO_PORTS: MemoryRange = 0xFF00..=0xFF4B;
	/// High RAM.
	pub const MMAP_RAM_HIGH: MemoryRange = 0xFF80..=0xFFFE;
	/// Interrupt enable register.
	pub const MMAP_INTERRUPT_EN: MemoryRange = 0xFFFF..=0xFFFF;
}

#[allow(unused_imports)]
use consts::*;

/// TODO decide on how to design this trait
pub trait Memory {
	/// TODO
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError>;

	/// TODO
	fn read(&self, address: u16) -> Result<u8, GameboyError>;
}

/// A virtual representation of Gameboy (Color) memory bus.
///
/// This implementation provides memory/peripheral abstraction.
pub struct SystemBus<'a> {
	//ram: Ram,
	cartridge: Cartridge<'a>,
}

/// An abstraction for fetching mutable and immutable regions.
macro_rules! get_region {
	($name:tt $(,$mut_:tt)*) => {
		/// Returns the region that contains the given address.
		fn $name(&$($mut_)* self, address: u16) -> Result<&$($mut_)* dyn Memory, GameboyError> {
			match address {
				_addr if MMAP_ROM_BANK0.contains(&_addr) => {
					Ok(&$($mut_)* self.cartridge)
				}
				// Switchable RAM bank
				// _addr if MMAP_RAM_BANK_SW.contains(&_addr) => {

				// }
				// Internal RAM
				// _addr if MMAP_RAM_INTERNAL.contains(&_addr) => {

				// }
				// Echo of internal RAM
				// _addr if MMAP_RAM_ECHO.contains(&_addr) => {

				// }
				_ => { Err(GameboyError::Io("Accessed unmapped region.")) }
			}
		}
	}
}

impl<'a> SystemBus<'a> {
	/// Initialize a new address space.
	pub fn new(cartridge: Cartridge<'a>) -> Self {
		SystemBus { cartridge }
	}

	// Get an immutable region
	get_region!(region);

	// Get a mutable region
	get_region!(region_mut, mut);
}

impl<'a> Memory for SystemBus<'a> {
	/// Handle reading from a memory region.
	///
	/// The function calls the relevent peripheral's implementation.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		let peripheral = self.region_mut(address)?;

		peripheral.write(address, value)
	}

	/// Handle writing to a memory region.
	///
	/// The function calls the relevent peripheral's implementation.
	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		let peripheral = self.region(address)?;
		
		peripheral.read(address)
	}
}

// pub struct Ram {

// }

// impl Memory for Ram {

// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_range() {
    	let int_enable_ptr: u16 = 0xFFFF;
    	let ram_ptr: u16 = 0xA100;

    	match int_enable_ptr {
    		_x if MMAP_INTERRUPT_EN.contains(&_x) => { }
    		_ => { assert!(false); }
    	}

    	match ram_ptr {
    		_x if MMAP_RAM_BANK_SW.contains(&_x) => { }
    		_ => { assert!(false); }
    	}
    }
}
