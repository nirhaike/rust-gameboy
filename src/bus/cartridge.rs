// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! The cartridge controller - lazy and zero-copy implementation for loading and
//! handling IO from/to the game's cartridge.

use crate::GameboyError;
use super::rtc::*;
use super::Memory;
use super::memory_range::*;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// cartridge addresses-related constants.
#[allow(missing_docs)]
pub mod consts {
	use super::*;

	/// The game's title string.
	pub const ROM_GAME_TITLE: MemoryRange = make_range!(0x0134, 0x0142);

	/// Gameboy color indicator.
	/// 0x80 for GBC, otherwise not.
	pub const ROM_GAMEBOY_COLOR: usize = 0x0143;
	/// Gameboy Super indicator.
	/// 0x03 for SGB, 0x00 for GB.
	pub const ROM_GAMEBOY_SUPER: usize = 0x0143;
	/// Cartridge type.
	///
	/// 0 - ROM Only, 1 - ROM+MBC1, 2 - ROM+MBC1+RAM, 3 - ROM+MBC1+RAM+Battery,
	/// 5 - ROM+MBC2, 6 - ROM+MBC2+Battery, 8 - ROM+RAM, 9 - ROM+RAM+Battery,
	/// 12 - ROM+MBC3+RAM, 13 - ROM+MBC3+RAM+Battery, 19 - ROM+MBC5,
	/// 1A - ROM+MBC5+RAM, 1B - ROM+MBC5+RAM+Battery, 1C - ROM+MBC5+Rumble,
	/// 1D - ROM+MBC5+Rumble+SRAM, 1E - ROM+MBC5+Rumble+SRAM+Battery
	pub const ROM_CARTRIDGE_TYPE: usize = 0x0147;

	/// The number of ROM banks in the cartridge.
	pub const ROM_SIZE: usize = 0x0148;

	/// The number of RAM banks supported in the cartridge.
	pub const RAM_SIZE: usize = 0x0149;

	/// A write to this range selects the memory model in MBC1 ROMs.
	pub const MEMORY_MODEL_SELECT: MemoryRange = make_range!(0x6000, 0x7FFF);

	/// A write to this range enables/disables the external RAM (and
	/// also the RTC's registers on MBC3 cartridges).
	pub const RAM_ENABLE_SELECT: MemoryRange = make_range!(0x0000, 0x1FFF);

	/// A write to this range selects the active ROM bank in MBC ROMs.
	pub const ROM_BANK_SELECT: MemoryRange = make_range!(0x2000, 0x3FFF);

	/// A write to this range selects the active RAM bank in MBC ROMs.
	pub const RAM_BANK_SELECT: MemoryRange = make_range!(0x4000, 0x5FFF);

	/// A write to this range fetches the current time into the RTC's registers.
	pub const CLOCK_DATA_LATCH: MemoryRange = make_range!(0x6000, 0x7FFF);
}

use consts::*;

/// Holds the cartridge's type and state.
#[derive(PartialEq)]
pub enum CartridgeType {
	/// A 32KB ROM, occupies 0000-7FFF.
	RomOnly,
	/// Memory bank controller 1.
	/// The ROM bank ranges from 0 to 3.
	///
	/// # Parameters
	///
	/// * Memory model select.
	MBC1(MemoryModel),
	/// Memory bank controller 2.
	/// The ROM bank ranges from 0 to 15.
	MBC2,
	/// Memory bank controller 3.
	/// This controlller also contains an RTC (real-time clock).
	/// The ROM bank ranges from 0 to 127.
	MBC3,
	/// Memory bank controller 5.
	/// This controller is guaranteed to run Gameboy Color games in double-speed mode.
	/// The ROM bank ranges from 0 to 127.
	MBC5,
}

/// Type-1 Memory bank controller has two models that determines the memory layout
/// at runtime.
#[derive(PartialEq)]
pub enum MemoryModel {
	/// 2MB ROM, 8KB RAM
	MoreRom,
	/// 0.5MB ROM, 32KB RAM
	MoreRam,
}

/// Cartridges with memory bank controllers are capable of swapping memory banks
/// by writing values to certain memory range within the cartridge.
///
/// This macro converts value written to the cartridge to the appropriate bank number.
#[allow(unused_macros)]
macro_rules! bank_number {
	($value:tt, $num_bits:tt) => (value & ((1 << $num_bits) - 1))
}

/// The game's cartridge
#[allow(dead_code)]
pub struct Cartridge<'a> {
	rom: &'a mut [u8],
	ram: &'a mut [u8],
	cart_type: CartridgeType,
	rtc: Rtc,
	rom_bank: u8,
	ram_bank: u8,
	ram_enabled: bool,
	rtc_mapped: bool,
}

impl<'a> Cartridge<'a> {
	/// Initialize a new cartridge given its raw data.
	pub fn new(rom: &'a mut [u8], ram: &'a mut [u8]) -> Result<Self, GameboyError> {
		// Make sure that the rom contains at least a single bank
		assert!(rom.len() >= 0x4000);
		assert!(ram.len() == Cartridge::ram_size(rom)?);

		// Find out the type of the cartridge
		let cart_type = match rom[ROM_CARTRIDGE_TYPE] {
			0x00 | 0x08 | 0x09 => CartridgeType::RomOnly,
			0x01 | 0x02 | 0x03 => CartridgeType::MBC1(MemoryModel::MoreRom),
			0x05 | 0x06 => CartridgeType::MBC2,
			0x12 | 0x13 => CartridgeType::MBC3,
			0x19 | 0x1A | 0x1C | 0x1D | 0x1E => CartridgeType::MBC5,
			_ => { return Err(GameboyError::Cartridge("Invalid cartridge type.")); }
		};

		let cart = Cartridge {
			rom,
			ram,
			cart_type,
			rtc: Rtc::new(),
			rom_bank: 0,
			ram_bank: 0,
			ram_enabled: false,
			rtc_mapped: false,
		};

		Ok(cart)
	}

	/// Get the title of the game.
	pub fn title(&'a self) -> &'a[u8] {
		&self.rom[memory_offset_range!(ROM_GAME_TITLE)]
	}

	/// Set the current active rom bank of the cartridge.
	///
	/// The command to set the rom bank is given by writing to a corresponding
	/// memory range.
	pub fn set_rom_bank(&mut self, address: u16, _value: u8) -> Result<(), GameboyError> {
		// TODO implement this. The implementation should depend on the cartridge type.
		match address {
			memory_range!(ROM_BANK_SELECT) => { unimplemented!(); }
			_ => { return Err(GameboyError::BadAddress(address)) }
		}
	}

	/// Set the current active ram bank of the cartridge.
	///
	/// The acctive ram bank is manipulated by programatically performing a write
	/// to the `RAM_BANK_SELECT` memory range.
	pub fn set_ram_bank(&mut self, _value: u8) -> Result<(), GameboyError> {
		// TODO implement this.
		unimplemented!();
	}

	/// Implementation of `write` for CartridgeType::RomOnly devices.
	fn write_romonly(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		// The memory model here must be RomOnly.
		assert!(CartridgeType::RomOnly == self.cart_type);

		// Make sure that the address is within our ROM bounds.
		if (address as usize) >= self.rom.len() {
			return Err(GameboyError::BadAddress(address));
		}
		self.rom[address as usize] = value;

		Ok(())
	}

	/// Implementation of `write` for CartridgeType::MBC1 devices.
	fn write_mbc1(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		// Get the MBC's current memory model
		let model_select: &mut MemoryModel = match self.cart_type {
			CartridgeType::MBC1(ref mut model) => { model }
			_ => {
				return Err(GameboyError::Cartridge("MBC1 memory model was expected."));
			}
		};
		// The write operation's implications depends on the address
		// that we're writing to, as some address ranges are reserved
		// for swapping memory model or changing the active rom bank.
		match address {
			memory_range!(MEMORY_MODEL_SELECT) => {
				// Change active memory model.
				*model_select = match value & 1 {
					0 => { MemoryModel::MoreRom }
					_ => { MemoryModel::MoreRam }
				};
				return Ok(());
			}
			memory_range!(ROM_BANK_SELECT) => {
				// Change active rom bank.
				self.set_rom_bank(address, value)?;
				return Ok(());
			}
			_ => {
				// The rest of the layout depends on the memory model.
				match model_select {
					MemoryModel::MoreRom => { unimplemented!(); }
					MemoryModel::MoreRam => { unimplemented!(); }
				}
			}
		}
	}

	/// Implementation of `write` for CartridgeType::MBC3 devices.
	fn write_mbc3(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		// The memory model here must be MBC3.
		assert!(CartridgeType::MBC3 == self.cart_type);

		// The consequences of the write operation depends on the written address.
		// There are addresses that are preserved for operations such as
		// changing ROM bank, etc.
		match address {
			memory_range!(RAM_ENABLE_SELECT) => {
				// Writing bits 1 and 3 to this range enables the ram and rtc registers,
				// otherwise they'll be disabled.
				self.ram_enabled = (value & 0x0A) != 0;
				return Ok(());
			}
			memory_range!(ROM_BANK_SELECT) => {
				// Change active rom bank.
				self.set_rom_bank(address, value)?;
				return Ok(());
			}
			memory_range!(RAM_BANK_SELECT) => {
				if RTC_CONTROL_RANGE.contains(&value) {
					// Change active rtc register.
					self.rtc.set_active_register(value)?;
					self.rtc_mapped = true;
				} else {
					// Change active ram bank.
					self.set_ram_bank(value)?;
					self.rtc_mapped = false;
				}
				return Ok(());
			}
			memory_range!(CLOCK_DATA_LATCH) => {
				// Update the clock's registers.
				self.rtc.latch();
				return Ok(());
			}

			_ => {
				// TODO implement reading rom & external ram.
				unimplemented!();
			}
		}
	}

	/// Get the number of ROM banks in the cartridge
	#[allow(dead_code)]
	fn num_rom_banks(&'a self) -> Result<u8, GameboyError> {
		let num_banks: u8 = match self.rom[ROM_SIZE] {
			0x00 => 2,  0x01 => 4,  0x02 => 8,   0x03 => 16,
			0x04 => 32, 0x05 => 64, 0x06 => 128, 0x52 => 72,
			0x53 => 80, 0x54 => 96,
			_ => {
				// Other values are generally not valid
				return Err(GameboyError::Cartridge("Invalid ROM banks configuration."));
			}
		};

		Ok(num_banks)
	}

	/// Get the supported RAM size in kilobytes given the relevant rom.
	fn ram_size(rom: &'a [u8]) -> Result<usize, GameboyError> {
		let num_banks: usize = match rom[ROM_SIZE] {
			0x00 => 0,
			0x01 => 0x800,
			0x02 => 0x2000,
			0x03 => 0x8000,
			0x04 => 0x20000,
			_ => {
				// Other values are generally not valid
				return Err(GameboyError::Cartridge("Invalid RAM banks configuration."));
			}
		};

		Ok(num_banks)
	}

	/// Create a ram buffer for the cartridge.
	#[inline(always)]
	#[allow(dead_code)]
	#[cfg(feature = "alloc")]
	fn make_ram(rom: &'a [u8]) -> Result<Box<[u8]>, GameboyError> {
		// We can't reuse the `ram_size` function as the array's size should be
		// statically determined.
		let ram: Box<[u8]> = match rom[ROM_SIZE] {
			0x00 => Box::new([0_u8; 0]),
			0x01 => Box::new([0_u8; 0x800]),
			0x02 => Box::new([0_u8; 0x2000]),
			0x03 => Box::new([0_u8; 0x8000]),
			0x04 => Box::new([0_u8; 0x20000]),
			_ => {
				return Err(GameboyError::Cartridge("Invalid number of RAM banks."));
			}
		};

		Ok(ram)
	}
}

impl<'a> Memory for Cartridge<'a> {
	/// Write data into the cartridge.
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		match self.cart_type {
			// No bank controller
			CartridgeType::RomOnly => {
				return self.write_romonly(address, value);
			}
			// Type-1 bank controller
			CartridgeType::MBC1(_) => {
				return self.write_mbc1(address, value);
			}
			// Type-3 bank controller
			CartridgeType::MBC3 => {
				return self.write_mbc3(address, value);
			}
			_ => {
				// These cartridge types are currently not implemented.
				return Err(GameboyError::NotImplemented);
			}
		}
	}

	/// Read data from the cartridge.
	fn read(&self, _address: u16) -> Result<u8, GameboyError> {
		unimplemented!();
	}
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
	use super::*;

	const TEST_GAME_TITLE: &[u8] = b"TEST TITLE\0\0\0\0\0";

	/// Creates an empty rom for testing.
	pub fn empty_rom() -> [u8; 0x8000] {
		let mut rom = [0_u8; 0x8000];
		// ROM-only cartridge.
		rom[ROM_CARTRIDGE_TYPE] = 0;
		// Write the game's title
		rom[memory_offset_range!(ROM_GAME_TITLE)].clone_from_slice(TEST_GAME_TITLE);

		rom
	}

	#[test]
	#[cfg(feature = "alloc")]
	fn test_cartridge_loading() -> Result<(), GameboyError> {
		let mut rom = empty_rom();
		let mut ram: Box<[u8]> = Cartridge::make_ram(&rom)?;

		let cart = Cartridge::new(&mut rom, &mut ram)?;

		// Make sure that the cartridge's API works as expected.
		assert!(CartridgeType::RomOnly == cart.cart_type);
		assert!(TEST_GAME_TITLE == cart.title());

		Ok(())
	}
}
