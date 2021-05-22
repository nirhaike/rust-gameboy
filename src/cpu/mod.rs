// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy's processor emulation.

pub mod state;
pub mod decode;
pub mod instructions;

use state::*;
use state::registers::*;
use instructions::Instruction;

use crate::bus::*;
use crate::GameboyError;
use crate::config::Config;
use crate::bus::cartridge::*;

/// The gameboy's processor.
///
/// This struct contains the complete emulator's state.
#[allow(dead_code)]
pub struct Cpu<'a> {
	// Interrupts, system tick, cpu speed, serial ports and etc. should come here

	/// The cpu's registers.
	registers: CpuState<'a>,
	/// The devices' memory mapping
	pub mmap: SystemBus<'a>,
	/// The emulator's configuration
	pub config: &'a Config,
}

impl<'a> Cpu<'a> {
	/// Initializes a new virtual cpu
	#[inline(always)]
	pub fn new(config: &'a Config, cartridge: &'a mut Cartridge<'a>) -> Self {
		Cpu {
			registers: CpuState::new(config),
			mmap: SystemBus::new(cartridge),
			config,
		}
	}

	/// Reads the next instruction byte and increments the program counter.
	pub fn fetch(&mut self) -> Result<u8, GameboyError> {
		let pc: u16 = self.registers.get(Register::PC);
		let insn_byte: u8 = self.mmap.read(pc)?;

		self.registers.set(Register::PC, pc + 1);

		Ok(insn_byte)
	}

	/// Emulates the execution of a single instruction.
	///
	/// Returns the number of clock cycles the instruction has taken.
	pub fn execute(&mut self) -> Result<usize, GameboyError> {
		// Handle interrupts.
		// TODO this.

		// Fetch the opcode from the memory.
		let opcode: u8 = self.fetch()?;

		// Decode the given opcode.
		// let insn: &Instruction = self.decode(opcode)?;
		let insn: Instruction = self.decode(opcode)?;

		// Execute and return the number of cycles taken.
		let num_cycles = insn(self)?;

		Ok(num_cycles)
	}
}
