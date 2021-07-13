// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy cpu's arithmetic and logic unit.

use super::Cpu;
use super::state::registers::*;
use super::instructions::InsnResult;

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
		let carry = cpu.registers.flag(Flag::C) as u8;

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
		let carry = cpu.registers.flag(Flag::C) as u16;

		let result_16 = (lhs as u16).wrapping_sub(rhs as u16).wrapping_sub(carry);
		let result: u8 = (result_16 & 0xFF) as u8;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, true);
		cpu.registers.set_flag(Flag::H, (lhs & 0x0F) < (rhs & 0x0F) + (carry as u8));
		cpu.registers.set_flag(Flag::C, (lhs as u16) < ((rhs as u16) + carry));

		result
	}

	/// Performs logical AND between the given arguments,
	/// sets the relevant flags accordinately and returns the result.
	pub fn and(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let result: u8 = lhs & rhs;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, true);
		cpu.registers.set_flag(Flag::C, false);

		result
	}

	/// Performs logical OR between the given arguments,
	/// sets the relevant flags accordinately and returns the result.
	pub fn or(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let result: u8 = lhs | rhs;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, false);

		result
	}

	/// Performs xor, sets the relevant flags accordinately and returns the result.
	pub fn xor(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		let result: u8 = lhs ^ rhs;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, false);

		result
	}

	/// Compares the given arguments and sets the relevant flags accordinately.
	pub fn cp(cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		// Compare is basically subtraction.
		sub(cpu, lhs, rhs)
	}

	/// Swaps the lower and highr nibble of the given value,
	/// and sets the relevant flags accordinately.
	pub fn swap(cpu: &mut Cpu, value: u8) -> u8 {
		let result: u8 = ((value & 0x0F) << 4) |
						 ((value & 0xF0) >> 4);

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, false);

		result
	}

	/// Rotates right the given register, possibly rotates the carry
	/// flag too (if !carry, the carry flag will hold bit 0's result, but
	/// bit 0 will move also to bit 7).
	pub fn rotate_right(cpu: &mut Cpu, value: u8, carry: bool) -> u8 {
		let old_carry = cpu.registers.flag(Flag::C);
		let new_carry = (value & 1) == 1;

		let mut result = value >> 1;

		if carry {
			result |= if old_carry { 0x80 } else { 0 };
		} else {
			result |= if new_carry { 0x80 } else { 0 };
		}

		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, new_carry);

		result
	}

	/// Rotates left the given register, possibly rotates the carry
	/// flag too (if !carry, the carry flag will hold bit 7's result, but
	/// bit 7 will move also to bit 0).
	pub fn rotate_left(cpu: &mut Cpu, value: u8, carry: bool) -> u8 {
		let old_carry = cpu.registers.flag(Flag::C);
		let new_carry = (value & 0x80) != 0;

		let mut result = value << 1;

		if carry {
			result |= if old_carry { 1 } else { 0 };
		} else {
			result |= if new_carry { 1 } else { 0 };
		}

		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, new_carry);

		result
	}

	/// Shifts right the given register. If logic (not arithmetic),
	/// the MSB shifts too, otherwise, the MSB stays the same.
	pub fn shift_right(cpu: &mut Cpu, value: u8, logic: bool) -> u8 {
		let old_msb = value & 0x80;
		let new_carry = (value & 1) == 1;

		let mut result: u8 = value >> 1;

		if !logic {
			result |= old_msb;
		}

		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, new_carry);

		result
	}

	/// Shifts left the given register.
	pub fn shift_left(cpu: &mut Cpu, value: u8) -> u8 {
		let new_carry = (value & 0x80) != 0;

		let result = value << 1;

		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, false);
		cpu.registers.set_flag(Flag::C, new_carry);

		result
	}

	/// Increment the given 8-bit register.
	pub fn inc_register(cpu: &mut Cpu, reg: Register) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		// Save the current carry flag.
		let old_carry = cpu.registers.flag(Flag::C);

		let value: u8 = cpu.registers.get(reg) as u8;
		let result: u8 = add(cpu, value, 1);

		cpu.registers.set(reg, result as u16);

		// Restore carry because inc shouldn't affect it.
		cpu.registers.set_flag(Flag::C, old_carry);

		Ok(4)
	}

	/// Increment the given 8-bit memory pointed by HL.
	pub fn inc_mem(cpu: &mut Cpu) -> InsnResult {
		let address = cpu.registers.get(Register::HL);

		// Save the current carry flag.
		let old_carry = cpu.registers.flag(Flag::C);

		let value: u8 = cpu.mmap.read(address)?;
		let result: u8 = add(cpu, value, 1);

		cpu.mmap.write(address, result)?;

		// Restore carry because inc shouldn't affect it.
		cpu.registers.set_flag(Flag::C, old_carry);

		Ok(12)
	}

	/// Decrement the given 8-bit register.
	pub fn dec_register(cpu: &mut Cpu, reg: Register) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		// Save the current carry flag.
		let old_carry = cpu.registers.flag(Flag::C);

		let value: u8 = cpu.registers.get(reg) as u8;
		let result: u8 = sub(cpu, value, 1);

		cpu.registers.set(reg, result as u16);

		// Restore carry because dec shouldn't affect it.
		cpu.registers.set_flag(Flag::C, old_carry);

		Ok(4)
	}

	/// Decrement the given 8-bit memory pointed by HL.
	pub fn dec_mem(cpu: &mut Cpu) -> InsnResult {
		let address = cpu.registers.get(Register::HL);

		// Save the current carry flag.
		let old_carry = cpu.registers.flag(Flag::C);

		let value: u8 = cpu.mmap.read(address)?;
		let result: u8 = sub(cpu, value, 1);

		cpu.mmap.write(address, result)?;

		// Restore carry because dec shouldn't affect it.
		cpu.registers.set_flag(Flag::C, old_carry);

		Ok(12)
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

/// Implementation of 16-bit arithmetic operations.
pub mod alu16 {
	use super::*;

	/// An ALU operation function pointer.
	pub type Alu16Op = fn(&mut Cpu, u16, u16) -> u16;

	/// Applies the given operation on two 16-bit registers.
	pub fn op_registers(
		op: Alu16Op,
		cpu: &mut Cpu,
		lhs: Register,
		rhs: Register) -> InsnResult
	{
		assert!(get_type(&lhs) == RegisterType::Wide);

		let left: u16 = cpu.registers.get(lhs);
		let right: u16 = cpu.registers.get(rhs);

		let result: u16 = op(cpu, left, right);

		// Store the result.
		cpu.registers.set(lhs, result);

		Ok(8)
	}

	/// Applies the given operation on a 16-bit register and 8-bit immediate.
	pub fn op_imm(
		op: Alu16Op,
		cpu: &mut Cpu,
		lhs: Register) -> InsnResult
	{
		assert!(get_type(&lhs) == RegisterType::Wide);

		let left: u16 = cpu.registers.get(lhs);
		let right: u16 = cpu.fetch::<u8>()? as u16;

		let result: u16 = op(cpu, left, right);

		// Store the result.
		cpu.registers.set(lhs, result);

		Ok(16)
	}

	/// Adds the given arguments, sets the relevant flags accordinately and returns the result.
	pub fn add(cpu: &mut Cpu, lhs: u16, rhs: u16) -> u16 {
		let result_32 = (lhs as u32).wrapping_add(rhs as u32);
		let result_16 = (lhs & 0x0FFF).wrapping_add(rhs & 0x0FFF);

		let result: u16 = (result_32 & 0xFFFF) as u16;

		// Set the relevant flags
		cpu.registers.set_flag(Flag::Z, result == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, result_16 > 0x0FFF);
		cpu.registers.set_flag(Flag::C, result_32 > 0xFFFF);

		result
	}

	/// Adds the given arguments, sets the relevant flags accordinately and returns the result.
	/// In this operation, the zero flag is not affected.
	pub fn add_hl(cpu: &mut Cpu, lhs: u16, rhs: u16) -> u16 {
		let result_32 = (lhs as u32).wrapping_add(rhs as u32);
		let result_16 = (lhs & 0x0FFF).wrapping_add(rhs & 0x0FFF);

		let result: u16 = (result_32 & 0xFFFF) as u16;

		// Set the relevant flags (the zero flag is not affected)
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, result_16 > 0x0FFF);
		cpu.registers.set_flag(Flag::C, result_32 > 0xFFFF);

		result
	}

	/// Increment the given 16-bit register.
	pub fn inc_register(cpu: &mut Cpu, reg: Register) -> InsnResult
	{
		assert!(get_type(&reg) == RegisterType::Wide);

		let value: u16 = cpu.registers.get(reg);
		let result: u16 = value.wrapping_add(1);

		cpu.registers.set(reg, result);

		Ok(8)
	}

	/// Decrement the given 16-bit register.
	pub fn dec_register(cpu: &mut Cpu, reg: Register) -> InsnResult
	{
		assert!(get_type(&reg) == RegisterType::Wide);

		let value: u16 = cpu.registers.get(reg);
		let result: u16 = value.wrapping_sub(1);

		cpu.registers.set(reg, result);

		Ok(8)
	}
}
