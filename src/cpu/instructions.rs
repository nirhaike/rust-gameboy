// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Implementation of the Z80-like cpu's instructions.

use super::Cpu;
use super::alu::*;
use super::state::registers::*;

use crate::GameboyError;

/// Instructions implementations returns the amount of cycles taken,
/// of the relevant error if occured.
pub type InsnResult = Result<usize, GameboyError>;
/// An instruction's method.
pub type Instruction = fn(&mut Cpu) -> InsnResult;

/// Internal utilities for implementing repeating logic once.
mod util {
	use super::*;

	/// Loads an 8-bit value into the given register.
	pub fn load_imm8_to_register(cpu: &mut Cpu,
								 reg: Register) -> InsnResult {

		assert!(get_type(&reg) != RegisterType::Wide);

		let value: u8 = cpu.fetch()?;
		cpu.registers.set(reg, value as u16);

		Ok(8)
	}

	/// Loads a 16-bit value into the given register.
	pub fn load_imm16_to_register(cpu: &mut Cpu,
								  reg: Register) -> InsnResult {

		assert!(get_type(&reg) == RegisterType::Wide);

		let value: u16 = cpu.fetch()?;
		cpu.registers.set(reg, value);

		Ok(12)
	}

	/// Moves the source register to the destination.
	pub fn move_registers(cpu: &mut Cpu,
						  dst: Register,
						  src: Register) -> InsnResult {

		assert!((get_type(&src) == RegisterType::Wide) ==
				(get_type(&dst) == RegisterType::Wide));

		let value = cpu.registers.get(src);
		cpu.registers.set(dst, value);

		// Wide registers moves are twice as long as short ones.
		if get_type(&dst) == RegisterType::Wide {
			Ok(8)
		} else {
			Ok(4)
		}
	}

	/// Reads the memory at address HL and stores the value to the
	/// given register.
	pub fn load_mem_to_register(cpu: &mut Cpu,
								reg: Register,
								mem: Register) -> InsnResult {
		assert!(get_type(&mem) == RegisterType::Wide);
		assert!(get_type(&reg) != RegisterType::Wide);

		let address = cpu.registers.get(mem);
		let value: u8 = cpu.mmap.read(address)?;
		cpu.registers.set(reg, value as u16);

		Ok(8)
	}

	/// Writes the given register's value to the memory at the address
	/// represented by the given 16-bit `mem` register (eg. HL).
	pub fn store_register_into_mem(cpu: &mut Cpu,
								   mem: Register,
								   reg: Register) -> InsnResult {
		assert!(get_type(&mem) == RegisterType::Wide);
		assert!(get_type(&reg) != RegisterType::Wide);

		let value: u8 = cpu.registers.get(reg) as u8;
		let address = cpu.registers.get(mem);

		cpu.mmap.write(address, value)?;

		Ok(8)
	}

	/// Places a 16-bit register on the stack.
	pub fn push_nn(cpu: &mut Cpu,
				   reg: Register) -> InsnResult {

		assert!(get_type(&reg) == RegisterType::Wide);

		let mut address: u16 = cpu.registers.get(Register::SP);
		let value: u16 = cpu.registers.get(reg);

		// Decrement the stack pointer.
		cpu.registers.set(Register::SP, address.wrapping_sub(2));

		address = address.wrapping_sub(1);
		cpu.mmap.write(address, ((value >> 8) & 0xFF) as u8)?;

		address = address.wrapping_sub(1);
		cpu.mmap.write(address, (value & 0xFF) as u8)?;

		Ok(16)
	}

	/// Pops a 16-bit register from the stack.
	pub fn pop_nn(cpu: &mut Cpu,
				  reg: Register) -> InsnResult {

		assert!(get_type(&reg) == RegisterType::Wide);

		let address: u16 = cpu.registers.get(Register::SP);

		let low = cpu.mmap.read(address)? as u16;
		let high = cpu.mmap.read(address.wrapping_add(1))? as u16;

		cpu.registers.set(reg, (high << 8) + low);

		// Increment the stack pointer.
		cpu.registers.set(Register::SP, address.wrapping_add(2));

		Ok(12)
	}

	pub fn jump_relative(cpu: &mut Cpu) -> InsnResult {
		let offset: i8 = cpu.fetch::<u8>()? as i8;
		let address: u16 = cpu.registers.get(Register::PC);

		// Add the offset to the program counter (preserving the offset's sign)
		cpu.registers.set(Register::PC, address.wrapping_add((offset as i16) as u16));

		Ok(8)
	}

	/// Performs a conditional jump instruction.
	pub fn jump_relative_conditional(cpu: &mut Cpu,
							flag: Flag,
							expected_state: bool) -> InsnResult {
		let offset: i8 = cpu.fetch::<u8>()? as i8;
		let address: u16 = cpu.registers.get(Register::PC);

		if cpu.registers.flag(flag) == expected_state {
			// Add the offset to the program counter (preserving the offset's sign)
			cpu.registers.set(Register::PC, address.wrapping_add((offset as i16) as u16));
		}

		Ok(8)
	}

	/// Performs an absolute jump instruction.
	pub fn jump_conditional(cpu: &mut Cpu,
							flag: Flag,
							expected_state: bool) -> InsnResult {
		let dest: u16 = cpu.fetch()?;

		if cpu.registers.flag(flag) == expected_state {
			cpu.registers.set(Register::PC, dest);
		}

		Ok(12)
	}

	/// Performs a conditional call instruction.
	pub fn call_conditional(cpu: &mut Cpu,
							flag: Flag,
							expected_state: bool) -> InsnResult {
		let dest: u16 = cpu.fetch()?;

		if cpu.registers.flag(flag) == expected_state {
			push_nn(cpu, Register::PC)?;
			cpu.registers.set(Register::PC, dest);
		}

		Ok(12)
	}

	pub fn ret_conditional(cpu: &mut Cpu,
						   flag: Flag,
						   expected_state: bool) -> InsnResult {

		if cpu.registers.flag(flag) == expected_state {
			pop_nn(cpu, Register::PC)?;
		}

		Ok(8)
	}

	/// Sets the flags according to the register's bit state.
	pub fn test_register_bit(cpu: &mut Cpu,
							 reg: Register,
							 bit: u8) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = (cpu.registers.get(reg) as u8) & (1 << bit);

		cpu.registers.set_flag(Flag::Z, data == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, true);
		// Carry is not affected.

		Ok(8)
	}

	/// Sets the flags according to the bit state of the data pointed by (HL).
	pub fn test_memory_bit(cpu: &mut Cpu,
						   bit: u8) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)? & (1 << bit);

		cpu.registers.set_flag(Flag::Z, data == 0);
		cpu.registers.set_flag(Flag::N, false);
		cpu.registers.set_flag(Flag::H, true);
		// Carry is not affected.

		Ok(16)
	}

	/// Resets the given bit of the given 8-bit register.
	pub fn reset_register_bit(cpu: &mut Cpu,
					 reg: Register,
					 bit: u8) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg);
		cpu.registers.set(reg, data & !(1 << bit));

		Ok(8)
	}

	/// Resets the given bit of the memory location pointer by (HL).
	pub fn reset_memory_bit(cpu: &mut Cpu, bit: u8) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		cpu.mmap.write(address, data & !(1 << bit))?;

		Ok(16)
	}

	/// Sets the given bit of the given 8-bit register.
	pub fn set_register_bit(cpu: &mut Cpu,
					 reg: Register,
					 bit: u8) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg);
		cpu.registers.set(reg, data | (1 << bit));

		Ok(8)
	}

	/// Sets the given bit of the memory location pointer by (HL).
	pub fn set_memory_bit(cpu: &mut Cpu, bit: u8) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		cpu.mmap.write(address, data | (1 << bit))?;

		Ok(16)
	}

	/// Swaps the nibbles of the given register.
	pub fn swap_register(cpu: &mut Cpu,
						 reg: Register) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg) as u8;

		let result = alu8::swap(cpu, data);
		cpu.registers.set(reg, result as u16);

		Ok(8)
	}

	/// Rotates right the given register, possibly rotates the carry
	/// flag too.
	pub fn rotate_right_register(cpu: &mut Cpu,
								 reg: Register,
								 carry: bool) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg) as u8;

		let result = alu8::rotate_right(cpu, data, carry);

		cpu.registers.set(reg, result as u16);

		Ok(8)
	}

	/// Rotates right the given memory data pointed by HL, possibly rotates
	/// the carry flag too.
	pub fn rotate_right_memory(cpu: &mut Cpu,
							   carry: bool) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		let result = alu8::rotate_right(cpu, data, carry);

		cpu.mmap.write(address, result)?;

		Ok(16)
	}

	/// Rotates left the given register, possibly rotates the carry
	/// flag too.
	pub fn rotate_left_register(cpu: &mut Cpu,
								reg: Register,
								carry: bool) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg) as u8;

		let result = alu8::rotate_left(cpu, data, carry);

		cpu.registers.set(reg, result as u16);

		Ok(8)
	}

	/// Rotates left the given memory data pointed by HL, possibly rotates
	/// the carry flag too.
	pub fn rotate_left_memory(cpu: &mut Cpu,
							  carry: bool) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		let result = alu8::rotate_left(cpu, data, carry);

		cpu.mmap.write(address, result)?;

		Ok(16)
	}

	/// Shifts right the given register.
	pub fn shift_right_register(cpu: &mut Cpu,
								 reg: Register,
								 logic: bool) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg) as u8;

		let result = alu8::shift_right(cpu, data, logic);

		cpu.registers.set(reg, result as u16);

		Ok(8)
	}

	/// Shifts right the given memory data pointed by HL.
	pub fn shift_right_memory(cpu: &mut Cpu,
							  logic: bool) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		let result = alu8::shift_right(cpu, data, logic);

		cpu.mmap.write(address, result)?;

		Ok(16)
	}

	/// Shifts left the given register.
	pub fn shift_left_register(cpu: &mut Cpu,
							   reg: Register) -> InsnResult {
		assert!(get_type(&reg) != RegisterType::Wide);

		let data = cpu.registers.get(reg) as u8;

		let result = alu8::shift_left(cpu, data);

		cpu.registers.set(reg, result as u16);

		Ok(8)
	}

	/// Shifts left the given memory data pointed by HL.
	pub fn shift_left_memory(cpu: &mut Cpu) -> InsnResult {
		let address = cpu.registers.get(Register::HL);
		let data = cpu.mmap.read(address)?;

		let result = alu8::shift_left(cpu, data);

		cpu.mmap.write(address, result)?;

		Ok(16)
	}

	pub fn restart(cpu: &mut Cpu, rst_vector: u16) -> InsnResult {
		push_nn(cpu, Register::PC)?;
		cpu.registers.set(Register::PC, rst_vector);

		Ok(32)
	}
}

use util::*;

/// Enter the given interrupt vector.
pub fn enter_interrupt(cpu: &mut Cpu, int_vector: u16) -> InsnResult {
	assert!(int_vector & 0xFF00 == 0);

	let cycles = push_nn(cpu, Register::PC)? + 8;

	// Disable interrupts, takes 4 cycles
	cpu.registers.set_ime(false);

	// Jump to the interrupt vector, takes 4 cycles.
	cpu.registers.set(Register::PC, int_vector);

	Ok(cycles)
}

/// nop
pub fn opcode_00(_cpu: &mut Cpu) -> InsnResult {
	Ok(4)
}

/// ld BC, nn
pub fn opcode_01(cpu: &mut Cpu) -> InsnResult {
	load_imm16_to_register(cpu, Register::BC)
}

/// ld (BC), A
pub fn opcode_02(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::BC, Register::A)
}

/// inc BC
pub fn opcode_03(cpu: &mut Cpu) -> InsnResult {
	alu16::inc_register(cpu, Register::BC)
}

/// inc B
pub fn opcode_04(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::B)
}

/// dec B
pub fn opcode_05(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::B)
}

/// ld B, n
pub fn opcode_06(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::B)
}

/// rlca
pub fn opcode_07(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::A, false)
}

/// ld (nn), SP
pub fn opcode_08(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.fetch()?;
	let value = cpu.registers.get(Register::SP);

	cpu.mmap.write(address, (value & 0xFF) as u8)?;
	cpu.mmap.write(address.wrapping_add(1), ((value >> 8) & 0xFF) as u8)?;

	Ok(20)
}

/// add HL, BC
pub fn opcode_09(cpu: &mut Cpu) -> InsnResult {
	alu16::op_registers(alu16::add_hl, cpu, Register::HL, Register::BC)
}

/// ld A, (BC)
pub fn opcode_0a(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::A, Register::BC)
}

/// dec BC
pub fn opcode_0b(cpu: &mut Cpu) -> InsnResult {
	alu16::dec_register(cpu, Register::BC)
}

/// inc C
pub fn opcode_0c(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::C)
}

/// dec C
pub fn opcode_0d(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::C)
}

/// ld C, n
pub fn opcode_0e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::C)
}

/// rrca
pub fn opcode_0f(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::A, false)
}

/// ld DE, nn
pub fn opcode_11(cpu: &mut Cpu) -> InsnResult {
	load_imm16_to_register(cpu, Register::DE)
}

/// ld (DE), A
pub fn opcode_12(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::DE, Register::A)
}

/// inc DE
pub fn opcode_13(cpu: &mut Cpu) -> InsnResult {
	alu16::inc_register(cpu, Register::DE)
}

/// inc D
pub fn opcode_14(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::D)
}

/// dec D
pub fn opcode_15(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::D)
}

/// ld D, n
pub fn opcode_16(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::D)
}

/// rla
pub fn opcode_17(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::A, true)
}

/// jr n
pub fn opcode_18(cpu: &mut Cpu) -> InsnResult {
	jump_relative(cpu)
}

/// add HL, DE
pub fn opcode_19(cpu: &mut Cpu) -> InsnResult {
	alu16::op_registers(alu16::add_hl, cpu, Register::HL, Register::DE)
}

/// ld A, (DE)
pub fn opcode_1a(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::A, Register::DE)
}

/// dec DE
pub fn opcode_1b(cpu: &mut Cpu) -> InsnResult {
	alu16::dec_register(cpu, Register::DE)
}

/// inc E
pub fn opcode_1c(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::E)
}

/// dec E
pub fn opcode_1d(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::E)
}

/// ld E, n
pub fn opcode_1e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::E)
}

/// rra
pub fn opcode_1f(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::A, true)
}


/// jr NZ, n
pub fn opcode_20(cpu: &mut Cpu) -> InsnResult {
	jump_relative_conditional(cpu, Flag::Z, false)
}

/// ld HL, nn
pub fn opcode_21(cpu: &mut Cpu) -> InsnResult {
	load_imm16_to_register(cpu, Register::HL)
}

/// ld (HL+), A
pub fn opcode_22(cpu: &mut Cpu) -> InsnResult {
	let address = cpu.registers.get(Register::HL);
	let value: u8 = cpu.registers.get(Register::A) as u8;

	// TODO remove this!
	#[cfg(feature = "debug")]
	{
		println!("Writing to 0x{:04x} value 0x{:02x}", address, value);
	}

	cpu.mmap.write(address, value)?;

	cpu.registers.set(Register::HL, address.wrapping_add(1));

	Ok(8)
}

/// inc HL
pub fn opcode_23(cpu: &mut Cpu) -> InsnResult {
	alu16::inc_register(cpu, Register::HL)
}

/// inc H
pub fn opcode_24(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::H)
}

/// dec H
pub fn opcode_25(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::H)
}

/// ld H, n
pub fn opcode_26(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::H)
}

/// jr Z, n
pub fn opcode_28(cpu: &mut Cpu) -> InsnResult {
	jump_relative_conditional(cpu, Flag::Z, true)
}

/// add HL, HL
pub fn opcode_29(cpu: &mut Cpu) -> InsnResult {
	alu16::op_registers(alu16::add_hl, cpu, Register::HL, Register::HL)
}

/// ld A, (HL+)
pub fn opcode_2a(cpu: &mut Cpu) -> InsnResult {
	let address = cpu.registers.get(Register::HL);
	let value: u8 = cpu.mmap.read(address)?;
	cpu.registers.set(Register::A, value as u16);
	cpu.registers.set(Register::HL, address.wrapping_add(1));

	Ok(8)
}

/// dec HL
pub fn opcode_2b(cpu: &mut Cpu) -> InsnResult {
	alu16::dec_register(cpu, Register::HL)
}

/// inc L
pub fn opcode_2c(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::L)
}

/// dec L
pub fn opcode_2d(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::L)
}

/// ld L, n
pub fn opcode_2e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::L)
}

/// cpl
pub fn opcode_2f(cpu: &mut Cpu) -> InsnResult {
	// Complement the A register.
	let value: u8 = cpu.registers.get(Register::A) as u8;
	cpu.registers.set(Register::A, (!value) as u16);

	Ok(4)
}

/// jr NC, n
pub fn opcode_30(cpu: &mut Cpu) -> InsnResult {
	jump_relative_conditional(cpu, Flag::C, false)
}

/// ld SP, nn
pub fn opcode_31(cpu: &mut Cpu) -> InsnResult {
	load_imm16_to_register(cpu, Register::SP)
}

/// ld (HL-), A
pub fn opcode_32(cpu: &mut Cpu) -> InsnResult {
	let address = cpu.registers.get(Register::HL);
	let value: u8 = cpu.registers.get(Register::A) as u8;

	cpu.mmap.write(address, value)?;

	cpu.registers.set(Register::HL, address.wrapping_sub(1));

	Ok(8)
}

/// inc SP
pub fn opcode_33(cpu: &mut Cpu) -> InsnResult {
	alu16::inc_register(cpu, Register::SP)
}

/// inc (HL)
pub fn opcode_34(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_mem(cpu)
}

/// dec (HL)
pub fn opcode_35(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_mem(cpu)
}

/// ld (HL), n
pub fn opcode_36(cpu: &mut Cpu) -> InsnResult {
	let value: u8 = cpu.fetch()?;
	let address = cpu.registers.get(Register::HL);

	cpu.mmap.write(address, value)?;

	Ok(12)
}

/// scf
pub fn opcode_37(cpu: &mut Cpu) -> InsnResult {
	// Set the carry flag.
	cpu.registers.set_flag(Flag::C, true);

	Ok(4)
}

/// jr C, n
pub fn opcode_38(cpu: &mut Cpu) -> InsnResult {
	jump_relative_conditional(cpu, Flag::C, true)
}

/// add HL, SP
pub fn opcode_39(cpu: &mut Cpu) -> InsnResult {
	alu16::op_registers(alu16::add_hl, cpu, Register::HL, Register::SP)
}

/// ld A, (HL-)
pub fn opcode_3a(cpu: &mut Cpu) -> InsnResult {
	let address = cpu.registers.get(Register::HL);
	let value: u8 = cpu.mmap.read(address)?;
	cpu.registers.set(Register::A, value as u16);
	cpu.registers.set(Register::HL, address.wrapping_sub(1));

	Ok(8)
}

/// dec SP
pub fn opcode_3b(cpu: &mut Cpu) -> InsnResult {
	alu16::dec_register(cpu, Register::SP)
}

/// inc A
pub fn opcode_3c(cpu: &mut Cpu) -> InsnResult {
	alu8::inc_register(cpu, Register::A)
}

/// dec A
pub fn opcode_3d(cpu: &mut Cpu) -> InsnResult {
	alu8::dec_register(cpu, Register::A)
}

/// ld A, #
pub fn opcode_3e(cpu: &mut Cpu) -> InsnResult {
	let value: u8 = cpu.fetch()?;
	cpu.registers.set(Register::A, value as u16);

	Ok(8)
}

/// ccf
pub fn opcode_3f(cpu: &mut Cpu) -> InsnResult {
	// Complement the carry flag.
	cpu.registers.set_flag(Flag::C, !cpu.registers.flag(Flag::C));

	Ok(4)
}

/// ld B, B
pub fn opcode_40(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::B)
}

/// ld B, C
pub fn opcode_41(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::C)
}

/// ld B, D
pub fn opcode_42(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::D)
}

/// ld B, E
pub fn opcode_43(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::E)
}

/// ld B, H
pub fn opcode_44(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::H)
}

/// ld B, L
pub fn opcode_45(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::L)
}

/// ld B, (HL)
pub fn opcode_46(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::B, Register::HL)
}

/// ld B, A
pub fn opcode_47(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::B, Register::A)
}

/// ld C, B
pub fn opcode_48(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::B)
}

/// ld C, C
pub fn opcode_49(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::C)
}

/// ld C, D
pub fn opcode_4a(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::D)
}

/// ld C, E
pub fn opcode_4b(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::E)
}

/// ld C, H
pub fn opcode_4c(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::H)
}

/// ld C, L
pub fn opcode_4d(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::L)
}

/// ld C, (HL)
pub fn opcode_4e(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::C, Register::HL)
}

/// ld C, A
pub fn opcode_4f(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::C, Register::A)
}

/// ld D, B
pub fn opcode_50(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::B)
}

/// ld D, C
pub fn opcode_51(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::C)
}

/// ld D, D
pub fn opcode_52(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::D)
}

/// ld D, E
pub fn opcode_53(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::E)
}

/// ld D, H
pub fn opcode_54(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::H)
}

/// ld D, L
pub fn opcode_55(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::L)
}

/// ld D, (HL)
pub fn opcode_56(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::D, Register::HL)
}

/// ld D, A
pub fn opcode_57(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::D, Register::A)
}

/// ld E, B
pub fn opcode_58(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::B)
}

/// ld E, C
pub fn opcode_59(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::C)
}

/// ld E, D
pub fn opcode_5a(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::D)
}

/// ld E, E
pub fn opcode_5b(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::E)
}

/// ld E, H
pub fn opcode_5c(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::H)
}

/// ld E, L
pub fn opcode_5d(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::L)
}

/// ld E, (HL)
pub fn opcode_5e(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::E, Register::HL)
}

/// ld E, A
pub fn opcode_5f(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::E, Register::A)
}

/// ld H, B
pub fn opcode_60(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::B)
}

/// ld H, C
pub fn opcode_61(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::C)
}

/// ld H, D
pub fn opcode_62(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::D)
}

/// ld H, E
pub fn opcode_63(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::E)
}

/// ld H, H
pub fn opcode_64(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::H)
}

/// ld H, L
pub fn opcode_65(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::L)
}

/// ld H, (HL)
pub fn opcode_66(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::H, Register::HL)
}

/// ld H, A
pub fn opcode_67(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::H, Register::A)
}

/// ld L, B
pub fn opcode_68(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::B)
}

/// ld L, C
pub fn opcode_69(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::C)
}

/// ld L, D
pub fn opcode_6a(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::D)
}

/// ld L, E
pub fn opcode_6b(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::E)
}

/// ld L, H
pub fn opcode_6c(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::H)
}

/// ld L, L
pub fn opcode_6d(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::L)
}

/// ld L, (HL)
pub fn opcode_6e(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::L, Register::HL)
}

/// ld L, A
pub fn opcode_6f(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::L, Register::A)
}

/// ld (HL), B
pub fn opcode_70(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::B)
}

/// ld (HL), C
pub fn opcode_71(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::C)
}

/// ld (HL), D
pub fn opcode_72(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::D)
}

/// ld (HL), E
pub fn opcode_73(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::E)
}

/// ld (HL), H
pub fn opcode_74(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::H)
}

/// ld (HL), L
pub fn opcode_75(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::L)
}

/// halt
pub fn opcode_76(cpu: &mut Cpu) -> InsnResult {
	cpu.halt();

	Ok(4)
}

/// ld (HL), A
pub fn opcode_77(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::HL, Register::A)
}

/// ld A, B
pub fn opcode_78(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::B)
}

/// ld A, C
pub fn opcode_79(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::C)
}

/// ld A, D
pub fn opcode_7a(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::D)
}

/// ld A, E
pub fn opcode_7b(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::E)
}

/// ld A, H
pub fn opcode_7c(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::H)
}

/// ld A, L
pub fn opcode_7d(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::L)
}

/// ld A, (HL)
pub fn opcode_7e(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::A, Register::HL)
}

/// ld A, A
pub fn opcode_7f(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::A, Register::A)
}

/// add A, B
pub fn opcode_80(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::B)
}

/// add A, C
pub fn opcode_81(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::C)
}

/// add A, D
pub fn opcode_82(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::D)
}

/// add A, E
pub fn opcode_83(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::E)
}

/// add A, H
pub fn opcode_84(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::H)
}

/// add A, L
pub fn opcode_85(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::L)
}

/// add A, (HL)
pub fn opcode_86(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::add, cpu)
}

/// add A, A
pub fn opcode_87(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::add, cpu, Register::A, Register::A)
}

/// adc A, B
pub fn opcode_88(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::B)
}

/// adc A, C
pub fn opcode_89(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::C)
}

/// adc A, D
pub fn opcode_8a(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::D)
}

/// adc A, E
pub fn opcode_8b(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::E)
}

/// adc A, H
pub fn opcode_8c(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::H)
}

/// adc A, L
pub fn opcode_8d(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::L)
}

/// adc A, (HL)
pub fn opcode_8e(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::adc, cpu)
}

/// adc A, A
pub fn opcode_8f(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::adc, cpu, Register::A, Register::A)
}

/// sub A, B
pub fn opcode_90(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::B)
}

/// sub A, C
pub fn opcode_91(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::C)
}

/// sub A, D
pub fn opcode_92(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::D)
}

/// sub A, E
pub fn opcode_93(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::E)
}

/// sub A, H
pub fn opcode_94(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::H)
}

/// sub A, L
pub fn opcode_95(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::L)
}

/// sub A, (HL)
pub fn opcode_96(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::sub, cpu)
}

/// sub A, A
pub fn opcode_97(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sub, cpu, Register::A, Register::A)
}

/// sbc A, B
pub fn opcode_98(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::B)
}

/// sbc A, C
pub fn opcode_99(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::C)
}

/// sbc A, D
pub fn opcode_9a(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::D)
}

/// sbc A, E
pub fn opcode_9b(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::E)
}

/// sbc A, H
pub fn opcode_9c(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::H)
}

/// sbc A, L
pub fn opcode_9d(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::L)
}

/// sbc A, (HL)
pub fn opcode_9e(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::sbc, cpu)
}

/// sbc A, A
pub fn opcode_9f(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::sbc, cpu, Register::A, Register::A)
}

/// and A, B
pub fn opcode_a0(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::B)
}

/// and A, C
pub fn opcode_a1(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::C)
}

/// and A, D
pub fn opcode_a2(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::D)
}

/// and A, E
pub fn opcode_a3(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::E)
}

/// and A, H
pub fn opcode_a4(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::H)
}

/// and A, L
pub fn opcode_a5(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::L)
}

/// and A, (HL)
pub fn opcode_a6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::and, cpu)
}

/// and A, A
pub fn opcode_a7(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::and, cpu, Register::A, Register::A)
}

/// xor A, B
pub fn opcode_a8(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::B)
}

/// xor A, C
pub fn opcode_a9(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::C)
}

/// xor A, D
pub fn opcode_aa(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::D)
}

/// xor A, E
pub fn opcode_ab(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::E)
}

/// xor A, H
pub fn opcode_ac(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::H)
}

/// xor A, L
pub fn opcode_ad(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::L)
}

/// xor A, (HL)
pub fn opcode_ae(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::xor, cpu)
}

/// xor A, A
pub fn opcode_af(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::xor, cpu, Register::A, Register::A)
}

/// or A, B
pub fn opcode_b0(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::B)
}

/// or A, C
pub fn opcode_b1(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::C)
}

/// or A, D
pub fn opcode_b2(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::D)
}

/// or A, E
pub fn opcode_b3(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::E)
}

/// or A, H
pub fn opcode_b4(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::H)
}

/// or A, L
pub fn opcode_b5(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::L)
}

/// or A, (HL)
pub fn opcode_b6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::or, cpu)
}

/// or A, A
pub fn opcode_b7(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::or, cpu, Register::A, Register::A)
}

/// cp A, B
pub fn opcode_b8(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::B)
}

/// cp A, C
pub fn opcode_b9(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::C)
}

/// cp A, D
pub fn opcode_ba(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::D)
}

/// cp A, E
pub fn opcode_bb(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::E)
}

/// cp A, H
pub fn opcode_bc(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::H)
}

/// cp A, L
pub fn opcode_bd(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::L)
}

/// cp A, (HL)
pub fn opcode_be(cpu: &mut Cpu) -> InsnResult {
	alu8::op_mem(alu8::cp, cpu)
}

/// cp A, A
pub fn opcode_bf(cpu: &mut Cpu) -> InsnResult {
	alu8::op_registers(alu8::cp, cpu, Register::A, Register::A)
}

/// ret NZ
pub fn opcode_c0(cpu: &mut Cpu) -> InsnResult {
	ret_conditional(cpu, Flag::Z, false)
}

/// pop BC
pub fn opcode_c1(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::BC)
}

/// jp NZ, nn
pub fn opcode_c2(cpu: &mut Cpu) -> InsnResult {
	jump_conditional(cpu, Flag::Z, false)
}

/// jp nn
pub fn opcode_c3(cpu: &mut Cpu) -> InsnResult {
	let dest: u16 = cpu.fetch()?;
	cpu.registers.set(Register::PC, dest);

	Ok(12)
}

/// call NZ, nn
pub fn opcode_c4(cpu: &mut Cpu) -> InsnResult {
	call_conditional(cpu, Flag::Z, false)
}

/// push BC
pub fn opcode_c5(cpu: &mut Cpu) -> InsnResult {
	push_nn(cpu, Register::BC)
}

/// add A, #
pub fn opcode_c6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::add, cpu)
}

/// rst 00h
pub fn opcode_c7(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x00)
}

/// ret Z
pub fn opcode_c8(cpu: &mut Cpu) -> InsnResult {
	ret_conditional(cpu, Flag::Z, true)
}

/// ret
pub fn opcode_c9(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::PC)?;

	Ok(8)
}

/// jp Z, nn
pub fn opcode_ca(cpu: &mut Cpu) -> InsnResult {
	jump_conditional(cpu, Flag::Z, true)
}

/// call Z, nn
pub fn opcode_cc(cpu: &mut Cpu) -> InsnResult {
	call_conditional(cpu, Flag::Z, true)
}

/// call nn
pub fn opcode_cd(cpu: &mut Cpu) -> InsnResult {
	let dest: u16 = cpu.fetch()?;

	push_nn(cpu, Register::PC)?;
	cpu.registers.set(Register::PC, dest);

	Ok(12)
}

/// adc A, #
pub fn opcode_ce(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::adc, cpu)
}

/// rst 08h
pub fn opcode_cf(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x08)
}

/// ret NC
pub fn opcode_d0(cpu: &mut Cpu) -> InsnResult {
	ret_conditional(cpu, Flag::C, false)
}

/// pop DE
pub fn opcode_d1(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::DE)
}

/// jp NC, nn
pub fn opcode_d2(cpu: &mut Cpu) -> InsnResult {
	jump_conditional(cpu, Flag::C, false)
}

/// call NC, nn
pub fn opcode_d4(cpu: &mut Cpu) -> InsnResult {
	call_conditional(cpu, Flag::C, false)
}

/// push DE
pub fn opcode_d5(cpu: &mut Cpu) -> InsnResult {
	push_nn(cpu, Register::DE)
}

/// sub A, #
pub fn opcode_d6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::sub, cpu)
}

/// rst 10h
pub fn opcode_d7(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x10)
}

/// ret C
pub fn opcode_d8(cpu: &mut Cpu) -> InsnResult {
	ret_conditional(cpu, Flag::C, true)
}

/// reti
pub fn opcode_d9(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::PC)?;

	cpu.registers.set_ime(true);

	Ok(8)
}

/// jp C, nn
pub fn opcode_da(cpu: &mut Cpu) -> InsnResult {
	jump_conditional(cpu, Flag::C, true)
}

/// call C, nn
pub fn opcode_dc(cpu: &mut Cpu) -> InsnResult {
	call_conditional(cpu, Flag::C, true)
}

/// sbc A, #
pub fn opcode_de(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::sbc, cpu)
}

/// rst 18h
pub fn opcode_df(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x18)
}

/// ld (n), A
pub fn opcode_e0(cpu: &mut Cpu) -> InsnResult {
	let low_byte = cpu.fetch::<u8>()? as u16;
	let address: u16 = 0xFF00 | low_byte;

	let value: u8 = cpu.registers.get(Register::A) as u8;

	// TODO remove this!
	#[cfg(feature = "debug")]
	{
		println!("Writing into 0x{:04x} value 0x{:02x}", address, value);
	}

	cpu.mmap.write(address, value)?;

	Ok(12)
}

/// pop HL
pub fn opcode_e1(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::HL)
}

/// ld (C), A
pub fn opcode_e2(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = 0xFF00 | cpu.registers.get(Register::C);
	let value: u8 = cpu.registers.get(Register::A) as u8;

	cpu.mmap.write(address, value)?;

	Ok(8)
}

/// push HL
pub fn opcode_e5(cpu: &mut Cpu) -> InsnResult {
	push_nn(cpu, Register::HL)
}

/// and A, #
pub fn opcode_e6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::and, cpu)
}

/// rst 20h
pub fn opcode_e7(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x20)
}

/// jp (HL)
pub fn opcode_e9(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.registers.get(Register::HL);

	cpu.registers.set(Register::PC, address);

	Ok(4)
}

/// ld (nn), A
pub fn opcode_ea(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.fetch::<u16>()?;
	let value: u8 = cpu.registers.get(Register::A) as u8;

	// TODO remove this!
	#[cfg(feature = "debug")]
	{
		println!("Writing to 0x{:04x} value 0x{:02x}", address, value);
	}

	cpu.mmap.write(address, value)?;

	Ok(16)
}

/// xor A, #
pub fn opcode_ee(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::xor, cpu)
}

/// rst 28h
pub fn opcode_ef(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x28)
}

/// ldh A, (n)
pub fn opcode_f0(cpu: &mut Cpu) -> InsnResult {
	let low_byte = cpu.fetch::<u8>()? as u16;
	let address: u16 = 0xFF00 | low_byte;

	let value: u8 = cpu.mmap.read(address)?;

	// TODO remove this!
	#[cfg(feature = "debug")]
	{
		println!("Reading from 0x{:04x} value 0x{:02x}", address, value);
	}

	cpu.registers.set(Register::A, value as u16);

	Ok(12)
}

/// pop AF
pub fn opcode_f1(cpu: &mut Cpu) -> InsnResult {
	pop_nn(cpu, Register::AF)
}

/// ld A, (C)
pub fn opcode_f2(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = 0xFF00 | cpu.registers.get(Register::C);
	let value: u8 = cpu.mmap.read(address)?;

	cpu.registers.set(Register::A, value as u16);

	Ok(8)
}

/// di
pub fn opcode_f3(cpu: &mut Cpu) -> InsnResult {
	cpu.registers.set_ime(false);

	Ok(4)
}

/// push AF
pub fn opcode_f5(cpu: &mut Cpu) -> InsnResult {
	push_nn(cpu, Register::AF)
}

/// or A, #
pub fn opcode_f6(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::or, cpu)
}

/// rst 30h
pub fn opcode_f7(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x30)
}

/// ld HL, SP+n
pub fn opcode_f8(cpu: &mut Cpu) -> InsnResult {
	let offset: u16 = cpu.fetch::<u8>()? as u16;
	let sp = cpu.registers.get(Register::SP);

	let result = alu16::add(cpu, sp, offset);

	cpu.registers.set(Register::HL, result);

	// According to the manual, this instruction always resets the zero flag.
	cpu.registers.set_flag(Flag::Z, false);

	Ok(12)
}

/// ld SP, HL
pub fn opcode_f9(cpu: &mut Cpu) -> InsnResult {
	move_registers(cpu, Register::SP, Register::HL)
}

/// ld A, (nn)
pub fn opcode_fa(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.fetch::<u16>()?;
	let value: u8 = cpu.mmap.read(address)?;

	cpu.registers.set(Register::A, value as u16);

	Ok(16)
}

/// ei
pub fn opcode_fb(cpu: &mut Cpu) -> InsnResult {
	cpu.toggle_ime_delayed();

	Ok(4)
}

/// cp A, #
pub fn opcode_fe(cpu: &mut Cpu) -> InsnResult {
	alu8::op_imm(alu8::cp, cpu)
}

/// rst 38h
pub fn opcode_ff(cpu: &mut Cpu) -> InsnResult {
	restart(cpu, 0x38)
}

/// rlc B
pub fn opcode_cb00(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::B, false)
}

/// rlc C
pub fn opcode_cb01(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::C, false)
}

/// rlc D
pub fn opcode_cb02(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::D, false)
}

/// rlc E
pub fn opcode_cb03(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::E, false)
}

/// rlc H
pub fn opcode_cb04(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::H, false)
}

/// rlc L
pub fn opcode_cb05(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::L, false)
}

/// rlc (HL)
pub fn opcode_cb06(cpu: &mut Cpu) -> InsnResult {
	rotate_left_memory(cpu, false)
}

/// rlc A
pub fn opcode_cb07(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::A, false)
}

/// rrc B
pub fn opcode_cb08(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::B, false)
}

/// rrc C
pub fn opcode_cb09(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::C, false)
}

/// rrc D
pub fn opcode_cb0a(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::D, false)
}

/// rrc E
pub fn opcode_cb0b(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::E, false)
}

/// rrc H
pub fn opcode_cb0c(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::H, false)
}

/// rrc L
pub fn opcode_cb0d(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::L, false)
}

/// rrc (HL)
pub fn opcode_cb0e(cpu: &mut Cpu) -> InsnResult {
	rotate_right_memory(cpu, false)
}

/// rrc A
pub fn opcode_cb0f(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::A, false)
}

/// rl B
pub fn opcode_cb10(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::B, true)
}

/// rl C
pub fn opcode_cb11(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::C, true)
}

/// rl D
pub fn opcode_cb12(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::D, true)
}

/// rl E
pub fn opcode_cb13(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::E, true)
}

/// rl H
pub fn opcode_cb14(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::H, true)
}

/// rl L
pub fn opcode_cb15(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::L, true)
}

/// rl (HL)
pub fn opcode_cb16(cpu: &mut Cpu) -> InsnResult {
	rotate_left_memory(cpu, true)
}

/// rl A
pub fn opcode_cb17(cpu: &mut Cpu) -> InsnResult {
	rotate_left_register(cpu, Register::A, true)
}

/// rr B
pub fn opcode_cb18(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::B, true)
}

/// rr C
pub fn opcode_cb19(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::C, true)
}

/// rr D
pub fn opcode_cb1a(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::D, true)
}

/// rr E
pub fn opcode_cb1b(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::E, true)
}

/// rr H
pub fn opcode_cb1c(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::H, true)
}

/// rr L
pub fn opcode_cb1d(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::L, true)
}

/// rr (HL)
pub fn opcode_cb1e(cpu: &mut Cpu) -> InsnResult {
	rotate_right_memory(cpu, true)
}

/// rr A
pub fn opcode_cb1f(cpu: &mut Cpu) -> InsnResult {
	rotate_right_register(cpu, Register::A, true)
}

/// sla B
pub fn opcode_cb20(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::B)
}

/// sla C
pub fn opcode_cb21(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::C)
}

/// sla D
pub fn opcode_cb22(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::D)
}

/// sla E
pub fn opcode_cb23(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::E)
}

/// sla H
pub fn opcode_cb24(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::H)
}

/// sla L
pub fn opcode_cb25(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::L)
}

/// sla (HL)
pub fn opcode_cb26(cpu: &mut Cpu) -> InsnResult {
	shift_left_memory(cpu)
}

/// sla A
pub fn opcode_cb27(cpu: &mut Cpu) -> InsnResult {
	shift_left_register(cpu, Register::A)
}

/// sra B
pub fn opcode_cb28(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::B, false)
}

/// sra C
pub fn opcode_cb29(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::C, false)
}

/// sra D
pub fn opcode_cb2a(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::D, false)
}

/// sra E
pub fn opcode_cb2b(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::E, false)
}

/// sra H
pub fn opcode_cb2c(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::H, false)
}

/// sra L
pub fn opcode_cb2d(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::L, false)
}

/// sra (HL)
pub fn opcode_cb2e(cpu: &mut Cpu) -> InsnResult {
	shift_right_memory(cpu, false)
}

/// sra A
pub fn opcode_cb2f(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::A, false)
}

/// swap B
pub fn opcode_cb30(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::B)
}

/// swap C
pub fn opcode_cb31(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::C)
}

/// swap D
pub fn opcode_cb32(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::D)
}

/// swap E
pub fn opcode_cb33(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::E)
}

/// swap H
pub fn opcode_cb34(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::H)
}

/// swap L
pub fn opcode_cb35(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::L)
}

/// swap (HL)
pub fn opcode_cb36(cpu: &mut Cpu) -> InsnResult {
	// Swap memory at (HL)
	let address: u16 = cpu.registers.get(Register::HL);
	let value: u8 = cpu.mmap.read(address)?;

	let result = alu8::swap(cpu, value);
	cpu.mmap.write(address, result)?;

	Ok(16)
}

/// swap A
pub fn opcode_cb37(cpu: &mut Cpu) -> InsnResult {
	swap_register(cpu, Register::A)
}

/// srl B
pub fn opcode_cb38(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::B, true)
}

/// srl C
pub fn opcode_cb39(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::C, true)
}

/// srl D
pub fn opcode_cb3a(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::D, true)
}

/// srl E
pub fn opcode_cb3b(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::E, true)
}

/// srl H
pub fn opcode_cb3c(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::H, true)
}

/// srl L
pub fn opcode_cb3d(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::L, true)
}

/// srl (HL)
pub fn opcode_cb3e(cpu: &mut Cpu) -> InsnResult {
	shift_right_memory(cpu, true)
}

/// srl A
pub fn opcode_cb3f(cpu: &mut Cpu) -> InsnResult {
	shift_right_register(cpu, Register::A, true)
}

/// bit 0, B
pub fn opcode_cb40(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 0)
}

/// bit 0, C
pub fn opcode_cb41(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 0)
}

/// bit 0, D
pub fn opcode_cb42(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 0)
}

/// bit 0, E
pub fn opcode_cb43(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 0)
}

/// bit 0, H
pub fn opcode_cb44(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 0)
}

/// bit 0, L
pub fn opcode_cb45(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 0)
}

/// bit 0, (HL)
pub fn opcode_cb46(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 0)
}

/// bit 0, A
pub fn opcode_cb47(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 0)
}

/// bit 1, B
pub fn opcode_cb48(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 1)
}

/// bit 1, C
pub fn opcode_cb49(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 1)
}

/// bit 1, D
pub fn opcode_cb4a(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 1)
}

/// bit 1, E
pub fn opcode_cb4b(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 1)
}

/// bit 1, H
pub fn opcode_cb4c(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 1)
}

/// bit 1, L
pub fn opcode_cb4d(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 1)
}

/// bit 1, (HL)
pub fn opcode_cb4e(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 1)
}

/// bit 1, A
pub fn opcode_cb4f(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 1)
}

/// bit 2, B
pub fn opcode_cb50(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 2)
}

/// bit 2, C
pub fn opcode_cb51(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 2)
}

/// bit 2, D
pub fn opcode_cb52(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 2)
}

/// bit 2, E
pub fn opcode_cb53(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 2)
}

/// bit 2, H
pub fn opcode_cb54(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 2)
}

/// bit 2, L
pub fn opcode_cb55(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 2)
}

/// bit 2, (HL)
pub fn opcode_cb56(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 2)
}

/// bit 2, A
pub fn opcode_cb57(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 2)
}

/// bit 3, B
pub fn opcode_cb58(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 3)
}

/// bit 3, C
pub fn opcode_cb59(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 3)
}

/// bit 3, D
pub fn opcode_cb5a(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 3)
}

/// bit 3, E
pub fn opcode_cb5b(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 3)
}

/// bit 3, H
pub fn opcode_cb5c(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 3)
}

/// bit 3, L
pub fn opcode_cb5d(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 3)
}

/// bit 3, (HL)
pub fn opcode_cb5e(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 3)
}

/// bit 3, A
pub fn opcode_cb5f(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 3)
}

/// bit 4, B
pub fn opcode_cb60(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 4)
}

/// bit 4, C
pub fn opcode_cb61(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 4)
}

/// bit 4, D
pub fn opcode_cb62(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 4)
}

/// bit 4, E
pub fn opcode_cb63(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 4)
}

/// bit 4, H
pub fn opcode_cb64(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 4)
}

/// bit 4, L
pub fn opcode_cb65(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 4)
}

/// bit 4, (HL)
pub fn opcode_cb66(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 4)
}

/// bit 4, A
pub fn opcode_cb67(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 4)
}

/// bit 5, B
pub fn opcode_cb68(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 5)
}

/// bit 5, C
pub fn opcode_cb69(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 5)
}

/// bit 5, D
pub fn opcode_cb6a(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 5)
}

/// bit 5, E
pub fn opcode_cb6b(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 5)
}

/// bit 5, H
pub fn opcode_cb6c(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 5)
}

/// bit 5, L
pub fn opcode_cb6d(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 5)
}

/// bit 5, (HL)
pub fn opcode_cb6e(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 5)
}

/// bit 5, A
pub fn opcode_cb6f(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 5)
}

/// bit 6, B
pub fn opcode_cb70(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 6)
}

/// bit 6, C
pub fn opcode_cb71(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 6)
}

/// bit 6, D
pub fn opcode_cb72(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 6)
}

/// bit 6, E
pub fn opcode_cb73(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 6)
}

/// bit 6, H
pub fn opcode_cb74(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 6)
}

/// bit 6, L
pub fn opcode_cb75(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 6)
}

/// bit 6, (HL)
pub fn opcode_cb76(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 6)
}

/// bit 6, A
pub fn opcode_cb77(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 6)
}

/// bit 7, B
pub fn opcode_cb78(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::B, 7)
}

/// bit 7, C
pub fn opcode_cb79(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::C, 7)
}

/// bit 7, D
pub fn opcode_cb7a(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::D, 7)
}

/// bit 7, E
pub fn opcode_cb7b(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::E, 7)
}

/// bit 7, H
pub fn opcode_cb7c(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::H, 7)
}

/// bit 7, L
pub fn opcode_cb7d(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::L, 7)
}

/// bit 7, (HL)
pub fn opcode_cb7e(cpu: &mut Cpu) -> InsnResult {
	test_memory_bit(cpu, 7)
}

/// bit 7, A
pub fn opcode_cb7f(cpu: &mut Cpu) -> InsnResult {
	test_register_bit(cpu, Register::A, 7)
}

/// res 0, B
pub fn opcode_cb80(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 0)
}

/// res 0, C
pub fn opcode_cb81(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 0)
}

/// res 0, D
pub fn opcode_cb82(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 0)
}

/// res 0, E
pub fn opcode_cb83(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 0)
}

/// res 0, H
pub fn opcode_cb84(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 0)
}

/// res 0, L
pub fn opcode_cb85(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 0)
}

/// res 0, (HL)
pub fn opcode_cb86(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 0)
}

/// res 0, A
pub fn opcode_cb87(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 0)
}

/// res 1, B
pub fn opcode_cb88(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 1)
}

/// res 1, C
pub fn opcode_cb89(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 1)
}

/// res 1, D
pub fn opcode_cb8a(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 1)
}

/// res 1, E
pub fn opcode_cb8b(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 1)
}

/// res 1, H
pub fn opcode_cb8c(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 1)
}

/// res 1, L
pub fn opcode_cb8d(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 1)
}

/// res 1, (HL)
pub fn opcode_cb8e(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 1)
}

/// res 1, A
pub fn opcode_cb8f(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 1)
}

/// res 2, B
pub fn opcode_cb90(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 2)
}

/// res 2, C
pub fn opcode_cb91(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 2)
}

/// res 2, D
pub fn opcode_cb92(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 2)
}

/// res 2, E
pub fn opcode_cb93(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 2)
}

/// res 2, H
pub fn opcode_cb94(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 2)
}

/// res 2, L
pub fn opcode_cb95(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 2)
}

/// res 2, (HL)
pub fn opcode_cb96(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 2)
}

/// res 2, A
pub fn opcode_cb97(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 2)
}

/// res 3, B
pub fn opcode_cb98(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 3)
}

/// res 3, C
pub fn opcode_cb99(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 3)
}

/// res 3, D
pub fn opcode_cb9a(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 3)
}

/// res 3, E
pub fn opcode_cb9b(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 3)
}

/// res 3, H
pub fn opcode_cb9c(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 3)
}

/// res 3, L
pub fn opcode_cb9d(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 3)
}

/// res 3, (HL)
pub fn opcode_cb9e(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 3)
}

/// res 3, A
pub fn opcode_cb9f(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 3)
}

/// res 4, B
pub fn opcode_cba0(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 4)
}

/// res 4, C
pub fn opcode_cba1(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 4)
}

/// res 4, D
pub fn opcode_cba2(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 4)
}

/// res 4, E
pub fn opcode_cba3(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 4)
}

/// res 4, H
pub fn opcode_cba4(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 4)
}

/// res 4, L
pub fn opcode_cba5(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 4)
}

/// res 4, (HL)
pub fn opcode_cba6(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 4)
}

/// res 4, A
pub fn opcode_cba7(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 4)
}

/// res 5, B
pub fn opcode_cba8(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 5)
}

/// res 5, C
pub fn opcode_cba9(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 5)
}

/// res 5, D
pub fn opcode_cbaa(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 5)
}

/// res 5, E
pub fn opcode_cbab(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 5)
}

/// res 5, H
pub fn opcode_cbac(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 5)
}

/// res 5, L
pub fn opcode_cbad(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 5)
}

/// res 5, (HL)
pub fn opcode_cbae(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 5)
}

/// res 5, A
pub fn opcode_cbaf(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 5)
}

/// res 6, B
pub fn opcode_cbb0(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 6)
}

/// res 6, C
pub fn opcode_cbb1(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 6)
}

/// res 6, D
pub fn opcode_cbb2(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 6)
}

/// res 6, E
pub fn opcode_cbb3(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 6)
}

/// res 6, H
pub fn opcode_cbb4(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 6)
}

/// res 6, L
pub fn opcode_cbb5(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 6)
}

/// res 6, (HL)
pub fn opcode_cbb6(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 6)
}

/// res 6, A
pub fn opcode_cbb7(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 6)
}

/// res 7, B
pub fn opcode_cbb8(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::B, 7)
}

/// res 7, C
pub fn opcode_cbb9(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::C, 7)
}

/// res 7, D
pub fn opcode_cbba(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::D, 7)
}

/// res 7, E
pub fn opcode_cbbb(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::E, 7)
}

/// res 7, H
pub fn opcode_cbbc(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::H, 7)
}

/// res 7, L
pub fn opcode_cbbd(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::L, 7)
}

/// res 7, (HL)
pub fn opcode_cbbe(cpu: &mut Cpu) -> InsnResult {
	reset_memory_bit(cpu, 7)
}

/// res 7, A
pub fn opcode_cbbf(cpu: &mut Cpu) -> InsnResult {
	reset_register_bit(cpu, Register::A, 7)
}

/// set 0, B
pub fn opcode_cbc0(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 0)
}

/// set 0, C
pub fn opcode_cbc1(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 0)
}

/// set 0, D
pub fn opcode_cbc2(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 0)
}

/// set 0, E
pub fn opcode_cbc3(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 0)
}

/// set 0, H
pub fn opcode_cbc4(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 0)
}

/// set 0, L
pub fn opcode_cbc5(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 0)
}

/// set 0, (HL)
pub fn opcode_cbc6(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 0)
}

/// set 0, A
pub fn opcode_cbc7(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 0)
}

/// set 1, B
pub fn opcode_cbc8(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 1)
}

/// set 1, C
pub fn opcode_cbc9(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 1)
}

/// set 1, D
pub fn opcode_cbca(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 1)
}

/// set 1, E
pub fn opcode_cbcb(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 1)
}

/// set 1, H
pub fn opcode_cbcc(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 1)
}

/// set 1, L
pub fn opcode_cbcd(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 1)
}

/// set 1, (HL)
pub fn opcode_cbce(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 1)
}

/// set 1, A
pub fn opcode_cbcf(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 1)
}

/// set 2, B
pub fn opcode_cbd0(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 2)
}

/// set 2, C
pub fn opcode_cbd1(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 2)
}

/// set 2, D
pub fn opcode_cbd2(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 2)
}

/// set 2, E
pub fn opcode_cbd3(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 2)
}

/// set 2, H
pub fn opcode_cbd4(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 2)
}

/// set 2, L
pub fn opcode_cbd5(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 2)
}

/// set 2, (HL)
pub fn opcode_cbd6(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 2)
}

/// set 2, A
pub fn opcode_cbd7(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 2)
}

/// set 3, B
pub fn opcode_cbd8(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 3)
}

/// set 3, C
pub fn opcode_cbd9(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 3)
}

/// set 3, D
pub fn opcode_cbda(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 3)
}

/// set 3, E
pub fn opcode_cbdb(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 3)
}

/// set 3, H
pub fn opcode_cbdc(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 3)
}

/// set 3, L
pub fn opcode_cbdd(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 3)
}

/// set 3, (HL)
pub fn opcode_cbde(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 3)
}

/// set 3, A
pub fn opcode_cbdf(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 3)
}

/// set 4, B
pub fn opcode_cbe0(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 4)
}

/// set 4, C
pub fn opcode_cbe1(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 4)
}

/// set 4, D
pub fn opcode_cbe2(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 4)
}

/// set 4, E
pub fn opcode_cbe3(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 4)
}

/// set 4, H
pub fn opcode_cbe4(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 4)
}

/// set 4, L
pub fn opcode_cbe5(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 4)
}

/// set 4, (HL)
pub fn opcode_cbe6(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 4)
}

/// set 4, A
pub fn opcode_cbe7(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 4)
}

/// set 5, B
pub fn opcode_cbe8(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 5)
}

/// set 5, C
pub fn opcode_cbe9(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 5)
}

/// set 5, D
pub fn opcode_cbea(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 5)
}

/// set 5, E
pub fn opcode_cbeb(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 5)
}

/// set 5, H
pub fn opcode_cbec(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 5)
}

/// set 5, L
pub fn opcode_cbed(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 5)
}

/// set 5, (HL)
pub fn opcode_cbee(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 5)
}

/// set 5, A
pub fn opcode_cbef(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 5)
}

/// set 6, B
pub fn opcode_cbf0(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 6)
}

/// set 6, C
pub fn opcode_cbf1(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 6)
}

/// set 6, D
pub fn opcode_cbf2(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 6)
}

/// set 6, E
pub fn opcode_cbf3(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 6)
}

/// set 6, H
pub fn opcode_cbf4(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 6)
}

/// set 6, L
pub fn opcode_cbf5(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 6)
}

/// set 6, (HL)
pub fn opcode_cbf6(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 6)
}

/// set 6, A
pub fn opcode_cbf7(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 6)
}

/// set 7, B
pub fn opcode_cbf8(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::B, 7)
}

/// set 7, C
pub fn opcode_cbf9(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::C, 7)
}

/// set 7, D
pub fn opcode_cbfa(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::D, 7)
}

/// set 7, E
pub fn opcode_cbfb(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::E, 7)
}

/// set 7, H
pub fn opcode_cbfc(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::H, 7)
}

/// set 7, L
pub fn opcode_cbfd(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::L, 7)
}

/// set 7, (HL)
pub fn opcode_cbfe(cpu: &mut Cpu) -> InsnResult {
	set_memory_bit(cpu, 7)
}

/// set 7, A
pub fn opcode_cbff(cpu: &mut Cpu) -> InsnResult {
	set_register_bit(cpu, Register::A, 7)
}

#[cfg(test)]
#[allow(dead_code)]
pub mod tests {
	use super::*;

	#[test]
	fn test_push_pop() -> Result<(), GameboyError> {
		super::super::tests::with_cpu(|cpu| {
			// Move the program counter to the RAM bank.
			cpu.registers.set(Register::PC, 0xA000);
			cpu.registers.set(Register::BC, 0x1234);

			// Write the opcodes the memory starting from the program counter.
			let data: &[u8] = &[/* PUSH BC */ 0xc5,
								/* POP BC  */ 0xc1];

			cpu.mmap.cartridge.set_ram_enabled(true);
			cpu.mmap.write_all(cpu.registers.get(Register::PC), data)?;

			cpu.execute_single()?;
			cpu.execute_single()?;

			// Make sure BC contains the same value.
			assert!(cpu.registers.get(Register::BC) == 0x1234);

			Ok(())
		})
	}

	#[test]
	fn test_jump_relative() -> Result<(), GameboyError> {
		super::super::tests::with_cpu(|cpu| {
			// Move the program counter to the RAM bank.
			cpu.registers.set(Register::PC, 0xA000);

			// Write the jump opcode
			let data: &[u8] = &[/* JR */ 0x18,
								/* -2 */ 0xfe];

			cpu.mmap.cartridge.set_ram_enabled(true);
			cpu.mmap.write_all(cpu.registers.get(Register::PC), data)?;

			cpu.execute_single()?;

			// Make sure BC contains the same value.
			assert!(cpu.registers.get(Register::PC) == 0xA000);

			Ok(())
		})
	}

	#[test]
	fn test_cpl() -> Result<(), GameboyError> {
		super::super::tests::with_cpu(|cpu| {
			// Move the program counter to the RAM bank.
			cpu.registers.set(Register::PC, 0xA000);
			cpu.registers.set(Register::AF, 0x1234);

			// Write the jump opcode
			let data: &[u8] = &[/* CPL */ 0x2f];

			cpu.mmap.cartridge.set_ram_enabled(true);
			cpu.mmap.write_all(cpu.registers.get(Register::PC), data)?;

			cpu.execute_single()?;

			// Make sure BC contains the same value.
			assert!(cpu.registers.get(Register::AF) == 0xed34);

			Ok(())
		})
	}

}
