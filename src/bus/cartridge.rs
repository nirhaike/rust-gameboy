// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! The cartridge controller - lazy and zero-copy implementation for loading and
//! handling IO from/to the game's cartridge.

use crate::GameboyError;
use super::Memory;
use super::memory_range::*;

/// cartridge addresses-related constants.
#[allow(missing_docs)]
pub mod consts {
	use super::*;

	/// Game title.
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
}

use consts::*;

/// Holds the cartridge's type and state.
#[derive(PartialEq)]
pub enum CartridgeType {
	/// A 32KB ROM, occupies 0000-7FFF.
	RomOnly,
	/// Memory bank controller 1.
	///
	/// * `M` - Memory model select.
	/// * `B` - Bank number (0 - 3).
	MBC1(/* M */ bool, /* B */ u8),
	/// Memory bank controller 2.
	///
	/// * `B` - Bank number (0 - 15).
	MBC2(/* B */ u8),
	/// Memory bank controller 3.
	///
	/// This controlller also contains an RTC (real-time clock).
	///
	/// * `B` - Bank number (0 - 127).
	MBC3(/* B */ u8),
	/// Memory bank controller 5.
	///
	/// This controller is guaranteed to run Gameboy Color games in double-speed mode.
	///
	/// * `B` - Bank number (0 - 127).
	MBC5(/* B */ u8),
}

/// The game's cartridge
pub struct Cartridge<'a> {
	data: &'a [u8],
	state: CartridgeType,
}

impl<'a> Cartridge<'a> {
	/// Initialize a new cartridge given its raw data.
	pub fn new(data: &'a [u8]) -> Self {
		// Make sure that the rom contains at least a single bank
		assert!(data.len() >= 0x4000);

		// Find out the type of the cartridge
		let state = match data[ROM_CARTRIDGE_TYPE] {
			0x00 | 0x08 | 0x09 => CartridgeType::RomOnly,
			0x01 | 0x02 | 0x03 => CartridgeType::MBC1(false, 0),
			0x05 | 0x06 => CartridgeType::MBC2(0),
			0x12 | 0x13 => CartridgeType::MBC3(0),
			0x19 | 0x1A | 0x1C | 0x1D | 0x1E => CartridgeType::MBC5(0),
			_ => CartridgeType::RomOnly,
		};

		Cartridge {
			data,
			state
		}
	}

	/// Get the title of the game.
	pub fn title(&self) -> &'a[u8] {
		&self.data[memory_offset_range!(ROM_GAME_TITLE)]
	}
}

impl<'a> Memory for Cartridge<'a> {
	/// TODO implement this.
	fn write(&mut self, _address: u16, _value: u8) -> Result<(), GameboyError> {
		Ok(())
	}

	/// TODO implement this.
	fn read(&self, _address: u16) -> Result<u8, GameboyError> {
		Ok(0)
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_GAME_TITLE: &[u8] = b"TEST TITLE\0\0\0\0\0";

    /// Creates an empty rom for testing.
    pub fn empty() -> [u8; 0x8000] {
    	let mut rom = [0_u8; 0x8000];
    	// ROM-only cartridge.
    	rom[ROM_CARTRIDGE_TYPE] = 0;
    	// Write the game's title
    	rom[memory_offset_range!(ROM_GAME_TITLE)].clone_from_slice(TEST_GAME_TITLE);

    	rom
    }

    #[test]
    fn test_cartridge_loading() {
    	let rom = empty();
    	let cart = Cartridge::new(&rom);

    	assert!(CartridgeType::RomOnly == cart.state);
    	assert!(TEST_GAME_TITLE == cart.title());
    }
}
