// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy's processor emulation.

pub mod state;
pub mod decode;
pub mod instructions;

use num::PrimInt;
use core::mem::size_of;
use core::ops::{AddAssign, Shl};

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

	/// Reads the next instruction bytes and increments the program counter appropriately.
	///
	/// The function works in little-endian, that is, when reading 2 bytes,
	/// the first byte will be the least-significant one.
	pub fn fetch<T: PrimInt + AddAssign + Shl<Output=T>>(&mut self) -> Result<T, GameboyError> {
		let mut result: T = num::cast(0).unwrap();

		for i in 0..size_of::<T>() {
			// Read the next byte.
			let pc: u16 = self.registers.get(Register::PC);
			let data: T = num::cast::<u8, T>(self.mmap.read(pc)?).unwrap();

			// We're using little-endianity.
			result += data << num::cast::<usize, T>(8 * i).unwrap();

			// Move the PC forward.
			self.registers.set(Register::PC, pc + 1);
		}

		Ok(result)
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

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
	use super::*;
	use alloc::boxed::Box;

	#[test]
	fn test_fetch() -> Result<(), GameboyError> {
		// Initialize the cpu
		let config = Config::default();
		let mut rom = cartridge::tests::empty_rom(CartridgeType::MBC3);
		let mut ram: Box<[u8]> = Cartridge::make_ram(&rom)?;
		let mut cartridge = Cartridge::new(&mut rom, &mut ram)?;

		let mut cpu = Cpu::new(&config, &mut cartridge);

		// Move the program counter to the RAM bank.
		cpu.registers.set(Register::PC, 0xA000);

		// Write arbitrary data to the memory starting from the program counter.
		let data: &[u8] = &[1, 2, 3];
		cpu.mmap.cartridge.set_ram_enabled(true);
		cpu.mmap.write_all(cpu.registers.get(Register::PC), data)?;

		// Make sure that fetch works as expected.
		assert!(cpu.fetch::<u16>()? == 0x0201);
		assert!(cpu.fetch::<u8>()? == 0x03);

		Ok(())
	}
}
