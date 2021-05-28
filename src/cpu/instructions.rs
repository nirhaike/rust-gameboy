// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Implementation of the Z80-like cpu's instructions.

use super::Cpu;
use super::state::registers::*;

use crate::bus::Memory;
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
		let value: u8 = cpu.fetch()?;
		cpu.registers.set(reg, value as u16);

		Ok(8)
	}

	/// Moves the source register to the destination.
	pub fn move_registers(cpu: &mut Cpu,
						  dst: Register,
						  src: Register) -> InsnResult {

		assert!(get_type(&src) == get_type(&dst));

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
}

use util::*;

/// ld (BC), A
pub fn opcode_02(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::BC, Register::A)
}

/// ld B, n
pub fn opcode_06(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::B)
}

/// ld A, (BC)
pub fn opcode_0a(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::A, Register::BC)
}

/// ld C, n
pub fn opcode_0e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::C)
}

/// ld (DE), A
pub fn opcode_12(cpu: &mut Cpu) -> InsnResult {
	store_register_into_mem(cpu, Register::DE, Register::A)
}

/// ld D, n
pub fn opcode_16(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::D)
}

/// ld A, (DE)
pub fn opcode_1a(cpu: &mut Cpu) -> InsnResult {
	load_mem_to_register(cpu, Register::A, Register::DE)
}

/// ld E, n
pub fn opcode_1e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::E)
}

/// ld H, n
pub fn opcode_26(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::H)
}

/// ld L, n
pub fn opcode_2e(cpu: &mut Cpu) -> InsnResult {
	load_imm8_to_register(cpu, Register::L)
}

/// ld (HL), n
pub fn opcode_36(cpu: &mut Cpu) -> InsnResult {
	let value: u8 = cpu.fetch()?;
	let address = cpu.registers.get(Register::HL);

	cpu.mmap.write(address, value)?;

	Ok(12)
}

/// ld A, #
pub fn opcode_3e(cpu: &mut Cpu) -> InsnResult {
	let value: u8 = cpu.fetch()?;
	cpu.registers.set(Register::A, value as u16);

	Ok(8)
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

/// ld (C), A
pub fn opcode_e2(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = 0xFF00 | cpu.registers.get(Register::C);
	let value: u8 = cpu.registers.get(Register::A) as u8;

	cpu.mmap.write(address, value)?;

	Ok(8)
}

/// ld (nn), A
pub fn opcode_ea(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.fetch::<u16>()?;
	let value: u8 = cpu.registers.get(Register::A) as u8;

	cpu.mmap.write(address, value)?;

	Ok(16)
}

/// ld A, (C)
pub fn opcode_f2(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = 0xFF00 | cpu.registers.get(Register::C);
	let value: u8 = cpu.mmap.read(address)?;

	cpu.registers.set(Register::A, value as u16);

	Ok(8)
}

/// ld A, (nn)
pub fn opcode_fa(cpu: &mut Cpu) -> InsnResult {
	let address: u16 = cpu.fetch::<u16>()?;
	let value: u8 = cpu.mmap.read(address)?;

	cpu.registers.set(Register::A, value as u16);

	Ok(16)
}
