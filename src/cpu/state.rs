// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Gameboy's processor state.

use crate::config::{Config, HardwareModel};
use registers::*;

#[allow(missing_docs)]
pub mod registers {
	/// The size of the register file
	pub const NUM_REGS: usize = 6;

	/// We have 6 registers and they're 16-bit wide.
	pub type RegisterFile = [u16; NUM_REGS];

	#[derive(PartialEq, Clone, Copy)]
	pub enum Register {
		/// Accumulator and Flag registers
		A, F, AF,
		B, C, BC,
		D, E, DE,
		/// Indirect access register
		H, L, HL,
		/// Stack pointer
		SP,
		/// Program counter
		PC,
	}

	/// The register's "type" is essentially the internal representation
	/// of the virtual register's bitmask within the register file.
	#[derive(PartialEq)]
	pub enum RegisterType {
		Wide,
		Low8,
		High8,
	}

	pub fn get_type(reg: &Register) -> RegisterType {
		match reg {
			Register::A |
			Register::B |
			Register::D |
			Register::H => RegisterType::High8,
			
			Register::F |
			Register::C |
			Register::E |
			Register::L => RegisterType::Low8,
			
			Register::AF |
			Register::BC |
			Register::DE |
			Register::HL |
			Register::SP |
			Register::PC => RegisterType::Wide,
		}
	}

	/// Get the index of a given register within the register file
	pub fn get_index(reg: &Register) -> usize {
		match reg {
			Register::A | Register::F | Register::AF => 0,
			Register::B | Register::C | Register::BC => 1,
			Register::D | Register::E | Register::DE => 2,
			Register::H | Register::L | Register::HL => 3,
			Register::SP => 4,
			Register::PC => 5,
		}
	}

	/// The flag register encodes the following flags within
	/// the register's bits.
	pub enum Flag {
		/// Carry flag
		C = 4,
		/// Half-Carry flag
		H = 5,
		/// Subtract flag
		N = 6,
		/// Zero flag
		Z = 7,

	}
}

/// Structure holding the current processor state.
#[derive(Clone)]
pub struct CpuState<'a> {
	regs: RegisterFile,
	config: &'a Config,
}

impl<'a> CpuState<'a> {
	/// Initializes a new cpu state
	pub fn new(config: &'a Config) -> Self {
		let mut state: CpuState<'a> = CpuState {
			regs: [0; NUM_REGS],
			config
		};

		// Reset the registers.
		state.reset();

		state
	}

	/// Reset registers to their initial boot state.
	pub fn reset(&mut self) {
		self.set(Register::F, 0xB0);
		self.set(Register::BC, 0x0013);
		self.set(Register::DE, 0x00D8);
		self.set(Register::HL, 0x014D);
		self.set(Register::SP, 0xFFFE);
		self.set(Register::PC, 0x0100);

		match self.config.model {
			HardwareModel::GB | HardwareModel::SGB => {
				self.set(Register::A, 0x01);
			},
			HardwareModel::GBC => {
				self.set(Register::A, 0x11);
			},
			HardwareModel::GBP => {
				self.set(Register::A, 0xFF);
			},
		}
	}

	/// Writes a value to a given register.
	///
	/// * `reg` - The register file identifier to write into.
	/// * `value` - The value to write. In cases of 8-bit register,
	///     the higher 8 bits will be discarded.
	pub fn set(&mut self, reg: Register, value: u16) {
		let reg_type: RegisterType = get_type(&reg);
		let reg: &mut u16 = &mut self.regs[get_index(&reg)];

		match reg_type {
			RegisterType::Wide => *reg = value,
			RegisterType::Low8 => *reg = (*reg & 0xFF00) | (value & 0x00FF),
			RegisterType::High8 => *reg = (*reg & 0x00FF) | ((value << 8) & 0xFF00),
		}
	}

	/// Reads the given register.
	pub fn get(&self, reg: Register) -> u16 {
		let reg_value: u16 = self.regs[get_index(&reg)];
		let reg_type: RegisterType = get_type(&reg);

		match reg_type {
			RegisterType::Wide => reg_value,
			RegisterType::Low8 => reg_value & 0x00FF,
			RegisterType::High8 => (reg_value >> 8) & 0x00FF,
		}
	}

	/// Returns the state of the given cpu flag, as stored in
	/// the 'F' register.
	pub fn get_flag(&self, flag: Flag) -> bool {
		let flags_value: u16 = self.get(Register::F);

		// Check whether the relevant bit is on
		((flags_value >> flag as u8) & 1) == 1
	}

	/// Returns the state of the given cpu flag, as stored in
	/// the 'F' register.
	pub fn set_flag(&mut self, flag: Flag, value: bool) {
		let old_flags: u16 = self.get(Register::F);

		let new_flags = if value {
			// Turn on the relevant bit
			old_flags | (1 << (flag as u8))
		} else {
			// Turn off the relevant bit
			old_flags & !(1 << (flag as u8))
		};

		self.set(Register::F, new_flags);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_registers_rw() {
		let cfg: &Config = &Config::default();
		let mut cpu: CpuState = CpuState::new(&cfg);

		assert_eq!(0x0013, cpu.get(Register::BC));

		cpu.set(Register::AF, 0x1234);
		assert_eq!(0x12, cpu.get(Register::A));
		assert_eq!(0x34, cpu.get(Register::F));

		cpu.set(Register::B, 0x18);
		assert_eq!(0x18, cpu.get(Register::B));

		cpu.set(Register::SP, 0x7FFC);
		assert_eq!(0x7FFC, cpu.get(Register::SP));
	}

	#[test]
	fn test_cpu_flags() {
		let cfg: &Config = &Config::default();
		let mut cpu: CpuState = CpuState::new(&cfg);

		cpu.set(Register::F, 0b10010000);
		//                    ^ZNHC
		assert_eq!(true, cpu.get_flag(Flag::Z) &&
						!cpu.get_flag(Flag::N) &&
						!cpu.get_flag(Flag::H) &&
						 cpu.get_flag(Flag::C));

		cpu.set(Register::F, 0b01000000);
		assert_eq!(true, !cpu.get_flag(Flag::Z) &&
						  cpu.get_flag(Flag::N) &&
						 !cpu.get_flag(Flag::H) &&
						 !cpu.get_flag(Flag::C));

		cpu.set_flag(Flag::N, false);
		assert_eq!(false, cpu.get_flag(Flag::N));

		cpu.set_flag(Flag::C, true);
		assert_eq!(true, cpu.get_flag(Flag::C));
	}
}
