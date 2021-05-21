// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! The emulation library's front-end API.

use crate::cpu::*;
use crate::bus::*;
use crate::bus::cartridge::*;
use crate::config::Config;

/// The complete emulator's state.
pub struct Emulator<'a> {
	// Interrupts, system tick, cpu speed, serial ports and etc. should come here

	/// The gameboy's processor
	pub cpu: Cpu<'a>,
	/// The devices' memory mapping
	pub mmap: SystemBus<'a>,
	/// The emulator's configuration
	pub config: &'a Config,
}

impl<'a> Emulator<'a> {
	/// Create a new emulator.
	#[inline(always)]
	pub fn new(config: &'a Config, cartridge: &'a mut Cartridge<'a>) -> Self {
		Emulator {
			cpu: Cpu::new(config),
			mmap: SystemBus::new(cartridge),
			config,
		}
	}
}
