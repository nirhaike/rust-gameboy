// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy's processor emulation.

pub mod state;

use state::*;
use crate::bus::*;
use crate::GameboyError;
use crate::config::Config;

/// The gameboy's processor.
#[allow(dead_code)]
pub struct Cpu<'a> {
	/// The cpu's registers.
	registers: CpuState<'a>,
}

impl<'a> Cpu<'a> {
	/// Initializes a new virtual cpu
	pub fn new(config: &'a Config) -> Self {
		Cpu {
			registers: CpuState::new(config),
		}
	}

	/// Emulates the execution of a single instruction.
	pub fn execute(_mmap: &'a mut SystemBus) -> Result<(), GameboyError> {
		// Handle interrupts.

		// Fetch the instruction from the memory.

		// Decode the given opcode.

		Ok(())
	}
}
