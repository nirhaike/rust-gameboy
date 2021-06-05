// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy cpu's arithmetic and logic unit.

use super::Cpu;
use super::state::registers::*;
use super::instructions::InsnResult;

use crate::bus::Memory;

/// Implementation of 8-bit arithmetic operations.
pub mod alu8 {
	use super::*;

	/// An ALU operation function pointer.
	pub type Alu8Op = fn(&mut Cpu, u8, u8) -> u8;

	/// Compare operations does not affect the lhs.
	macro_rules! stores_result {
		($op:tt) => (($op as usize) != (cp as usize))
	}

	/// Applies the given operation on two 8-bit registers.
	pub fn op_registers(
		op: Alu8Op,
		cpu: &mut Cpu,
		lhs: Register,
		rhs: Register) -> InsnResult
	{
		assert!(get_type(&lhs) != RegisterType::Wide);
		assert!(get_type(&rhs) != RegisterType::Wide);

		let left = cpu.registers.get(lhs) as u8;
		let right = cpu.registers.get(rhs) as u8;
		let result = op(cpu, left, right) as u16;

		if stores_result!(op) {
			cpu.registers.set(lhs, result);
		}

		Ok(4)
	}

	/// Applies the given operation on the A register and the given 8-bit immediate.
	pub fn op_imm(op: Alu8Op, cpu: &mut Cpu) -> InsnResult {
		let left = cpu.registers.get(Register::A) as u8;
		let imm = cpu.fetch::<u8>()?;

		let result = op(cpu, left, imm) as u16;

		if stores_result!(op) {
			cpu.registers.set(Register::A, result);
		}

		Ok(8)
	}

	/// Applies the given operation on the A register and the value at (HL).
	pub fn op_mem(op: Alu8Op, cpu: &mut Cpu) -> InsnResult {
		let address = cpu.registers.get(Register::HL);

		let left = cpu.registers.get(Register::A) as u8;
		let right: u8 = cpu.mmap.read(address)?;

		let result = op(cpu, left, right) as u16;

		if stores_result!(op) {
			cpu.registers.set(Register::A, result);
		}

		Ok(8)
	}

	/// Adds the given arguments, sets the relevant flags accordinately and returns the result.
	pub fn add(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let result_16 = (lhs as u16).wrapping_add(rhs as u16);
		let result_8 = (lhs & 0x0F).wrapping_add(rhs & 0x0F);

		let result: u8 = (result_16 & 0xFF) as u8;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, result_8 > 0x0F);
		cpu.registers.set_flag(Flag::C, result_16 > 0xFF);

		result
	}

	/// Adds the given arguments and the carry flag, if set.
	/// The function sets the relevant flags accordinately and returns the result.
	pub fn adc(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let carry = cpu.registers.get_flag(Flag::C) as u8;

		let result_16 = (lhs as u16).wrapping_add(rhs as u16).wrapping_add(carry as u16);
		let result_8 = (lhs & 0x0F).wrapping_add(rhs & 0x0F).wrapping_add(carry);

		let result: u8 = (result_16 & 0xFF) as u8;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, result_8 > 0x0F);
		cpu.registers.set_flag(Flag::C, result_16 > 0xFF);

		result
	}

	/// Subtracts the given arguments, sets the relevant flags accordinately and returns the result.
	pub fn sub(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let result_16 = (lhs as u16).wrapping_sub(rhs as u16);
		let result: u8 = (result_16 & 0xFF) as u8;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, true);
		cpu.registers.set_flag(Flag::H, (lhs & 0x0F) < (rhs & 0x0F));
		cpu.registers.set_flag(Flag::C, (lhs as u16) < (rhs as u16));

		result
	}

	/// Subtracts with carry, sets the relevant flags accordinately and returns the result.
	pub fn sbc(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let carry = cpu.registers.get_flag(Flag::C) as u16;

		let result_16 = (lhs as u16).wrapping_sub(rhs as u16).wrapping_sub(carry);
		let result: u8 = (result_16 & 0xFF) as u8;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, true);
		cpu.registers.set_flag(Flag::H, (lhs & 0x0F) < (rhs & 0x0F) + (carry as u8));
		cpu.registers.set_flag(Flag::C, (lhs as u16) < (rhs as u16) + carry);

		result
	}

	/// Compares the given arguments and sets the relevant flags accordinately.
	pub fn cp(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		// Compare is basically subtraction.
		sub(cpu, lhs, rhs)
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::GameboyError;

		/// Checks whether the correct instructions stores the operation results.
		#[test]
		fn test_writeback() -> Result<(), GameboyError> {
			let cp_ptr: Alu8Op = cp;
			let sub_ptr: Alu8Op = sub;

			assert!(!stores_result!(cp_ptr));
			assert!(stores_result!(sub_ptr));

			Ok(())
		}
	}
}
