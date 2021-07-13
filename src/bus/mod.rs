// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulate the gameboy's memory mapping and bus access.

#[macro_use]
pub mod memory_range;
pub mod cartridge;
pub mod joypad;
pub mod timer;
pub mod rtc;
pub mod ram;
pub mod ppu;
pub mod io;

use io::*;
use ram::*;
use ppu::*;
use timer::*;
use joypad::*;
use cartridge::*;
use memory_range::*;
use timer::consts::MMAP_IO_TIMER;
use ppu::consts::{MMAP_IO_DISPLAY, MMAP_IO_PALETTES};

use crate::GameboyError;
use crate::config::Config;
use crate::cpu::interrupts::*;

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
	pub const MMAP_IO_PORTS: MemoryRange = make_range!(0xFF00, 0xFF7F);
	/// High RAM.
	pub const MMAP_RAM_HIGH: MemoryRange = make_range!(0xFF80, 0xFFFE);
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
	pub(crate) ppu: Ppu,
	pub(crate) io: IoPorts,
	pub(crate) timer: Timer,
	pub(crate) joypad: Joypad,
	pub(crate) ram: InternalRam,

	/// The IF register.
	pub interrupt_flag: InterruptMask,
	/// The IE register.
	pub interrupt_enable: InterruptMask,
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
				memory_range!(MMAP_RAM_ECHO) |
				memory_range!(MMAP_RAM_HIGH) => {
					Ok(&$($mut_)* self.ram)
				}

				// Timer
				memory_range!(MMAP_IO_TIMER) => {
					Ok(&$($mut_)* self.timer)
				}

				// DMA and internal IO registers
				io::consts::IO_DMA |
				io::consts::IO_IF |
				io::consts::IO_IE => {
					Ok(&$($mut_)* *self)
				}

				// Display
				memory_range!(MMAP_IO_DISPLAY) |
				memory_range!(MMAP_IO_PALETTES) |
				memory_range!(MMAP_VIDEO_RAM) |
				memory_range!(MMAP_SPRITE_OAM) => {
					Ok(&$($mut_)* self.ppu)
				}

				// Joypad
				joypad::consts::IO_P1 => {
					Ok(&$($mut_)* self.joypad)
				}

				// I/O registers
				memory_range!(MMAP_IO_PORTS) => {
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
			ppu: Ppu::new(),
			io: IoPorts::new(config),
			timer: Timer::new(config),
			joypad: Joypad::new(),
			ram: InternalRam::new(),
			interrupt_flag: 0,
			interrupt_enable: 0,
		}
	}

	/// Update the system bus peripehrals' state according to
	/// the elapsed time.
	pub fn process(&mut self, cycles: usize) {
		let elapsed = if cycles > 0 { cycles } else { 4 };

		self.ppu.process(elapsed);
		self.timer.process(elapsed);
		self.joypad.process(elapsed);

		// Update interrupts state
		self.interrupt_flag |= self.ppu.interrupts();
		self.interrupt_flag |= self.timer.interrupts();
		self.interrupt_flag |= self.joypad.interrupts();
		self.interrupt_flag &= self.interrupt_enable;

		self.ppu.clear();
		self.timer.clear();
		self.joypad.clear();
	}

	/// Handle reading from a memory region.
	/// The function calls the relevent peripheral's implementation.
	pub fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		let peripheral = self.region_mut(address)?;

		peripheral.write(address, value)
	}

	/// Handle writing to a memory region.
	/// The function calls the relevent peripheral's implementation.
	pub fn read(&self, address: u16) -> Result<u8, GameboyError> {
		let peripheral = self.region(address)?;
		
		peripheral.read(address)
	}

	/// Returns a waiting interrupt and removes it from the queue.
	pub fn fetch_interrupt(&mut self) -> Option<Interrupt> {
		let mut iter = InterruptIter::new(self.interrupt_flag);
		let interrupt = iter.next();

		// Remove the fetched interrupt (if any) from the interrupt register.
		self.interrupt_flag = iter.mask;

		interrupt
	}

	// Get an immutable region
	get_region!(region);

	// Get a mutable region
	get_region!(region_mut, mut);
}

/// Certain registers needs access to multiple peripherals.
/// These registers will be implemented here.
mod private {
	use super::*;

	// Implement read/write operations for internal registers.
	impl<'a> Memory for SystemBus<'a> {

		fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
			match address {
				io::consts::IO_DMA => {
					// The (non-GBC's double-speed) clock speed is 4.194304 MHz.
					// It means that every cycle takes roughly 0.238419 microseconds.
					// DMA transfer takes 152 microseconds, meaning that it takes ~640 clock cycles.
					// The cycle-accurate gameboy docs describes the operation precisely.

					// TODO we need to make the dma transfer realistic instead of performing
					// it immediately, and allowing copy only from permitted addresses.
					let source: u16 = (value as u16) << 8;

					// Perform the transfer.
					for i in 0..0xa0 {
						let data = self.read(source + (i as u16))?;
						self.ppu.oam()[i] = data;
					}

					Ok(())
				}
				io::consts::IO_IF => {
					self.interrupt_flag = value;

					Ok(())
				}
				io::consts::IO_IE => {
					self.interrupt_enable = value;

					Ok(())
				}
				_ => {
					panic!("Write operation not implemented for register: {}", address);
				}
			}
		}

		fn read(&self, address: u16) -> Result<u8, GameboyError> {
			match address {
				io::consts::IO_DMA => {
					Ok(0)
				}
				io::consts::IO_IF => {
					Ok(self.interrupt_flag)
				}
				io::consts::IO_IE => {
					Ok(self.interrupt_enable)
				}
				_ => {
					panic!("Read operation not implemented for register: {}", address);
				}
			}
		}
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
