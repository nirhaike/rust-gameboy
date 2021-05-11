// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! The cartridge controller - lazy and zero-copy implementation for loading and
//! handling IO from/to the game's cartridge.

use crate::bus::Memory;
use crate::GameboyError;

/// Cartridge type
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
}

impl<'a> Cartridge<'a> {
	/// Initialize a new cartridge given its raw data.
	pub fn new(data: &'a [u8]) -> Self {
		Cartridge {
			data
		}
	}
}

impl<'a> Memory for Cartridge<'a> {
	/// TODO this.
	fn write(&mut self, _address: u16, _value: u8) -> Result<(), GameboyError> {
		Ok(())
	}

	/// TODO this.
	fn read(&self, _address: u16) -> Result<u8, GameboyError> {
		Ok(0)
	}
}
