// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulate the gameboy's memory mapping and bus access.

/// Bus locations-related constants.
#[allow(missing_docs)]
pub mod consts {
	pub const MAIN_RAM: u16 = 0x0000;
}

#[allow(unused_imports)]
use consts::*;

/// A virtual representation of Gameboy (Color) memory bus.
///
/// This implementation provides memory/peripheral abstraction.
pub struct SystemBus {

}

impl SystemBus {
	/// Initialize an address space.
	pub fn new() -> Self {
		SystemBus {}
	}
}

/// TODO decide on how to design this trait
pub trait MemoryRegion {
	/// TODO
	fn write(&self, address: u16, value: u8);

	/// TODO
	fn read(&self, address: u16) -> u8;

}

