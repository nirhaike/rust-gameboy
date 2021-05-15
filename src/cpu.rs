// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy's processor emulation.

use crate::state::*;
use crate::bus::*;
//use crate::config::Config;

/// TODO documentation
pub struct Cpu<'a> {
	// Interrupts, system tick, cpu speed, serial ports and etc. should come here

	/// The cpu's registers
	pub state: CpuState<'a>,
	/// The device's memory mapping
	pub mmap: SystemBus<'a>,
}
