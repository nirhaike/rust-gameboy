// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy cpu's opcode decoder.

use super::Cpu;
use super::instructions::*;
use crate::GameboyError;

impl<'a> Cpu<'a> {

	/// Returns the instruction that matches the given opcode.
	pub fn decode(&mut self, opcode: u8) -> Result<Instruction, GameboyError> {
		match opcode {
			0x01 => Ok(opcode_01),
			0x02 => Ok(opcode_02),
			0x06 => Ok(opcode_06),
			0x08 => Ok(opcode_08),
			0x0a => Ok(opcode_0a),
			0x0e => Ok(opcode_0e),
			0x11 => Ok(opcode_11),
			0x12 => Ok(opcode_12),
			0x16 => Ok(opcode_16),
			0x1a => Ok(opcode_1a),
			0x1e => Ok(opcode_1e),
			0x21 => Ok(opcode_21),
			0x22 => Ok(opcode_22),
			0x26 => Ok(opcode_26),
			0x2a => Ok(opcode_2a),
			0x2e => Ok(opcode_2e),
			0x31 => Ok(opcode_31),
			0x32 => Ok(opcode_32),
			0x36 => Ok(opcode_36),
			0x3a => Ok(opcode_3a),
			0x3e => Ok(opcode_3e),
			0x40 => Ok(opcode_40),
			0x41 => Ok(opcode_41),
			0x42 => Ok(opcode_42),
			0x43 => Ok(opcode_43),
			0x44 => Ok(opcode_44),
			0x45 => Ok(opcode_45),
			0x46 => Ok(opcode_46),
			0x47 => Ok(opcode_47),
			0x48 => Ok(opcode_48),
			0x49 => Ok(opcode_49),
			0x4a => Ok(opcode_4a),
			0x4b => Ok(opcode_4b),
			0x4c => Ok(opcode_4c),
			0x4d => Ok(opcode_4d),
			0x4e => Ok(opcode_4e),
			0x4f => Ok(opcode_4f),
			0x50 => Ok(opcode_50),
			0x51 => Ok(opcode_51),
			0x52 => Ok(opcode_52),
			0x53 => Ok(opcode_53),
			0x54 => Ok(opcode_54),
			0x55 => Ok(opcode_55),
			0x56 => Ok(opcode_56),
			0x57 => Ok(opcode_57),
			0x58 => Ok(opcode_58),
			0x59 => Ok(opcode_59),
			0x5a => Ok(opcode_5a),
			0x5b => Ok(opcode_5b),
			0x5c => Ok(opcode_5c),
			0x5d => Ok(opcode_5d),
			0x5e => Ok(opcode_5e),
			0x5f => Ok(opcode_5f),
			0x60 => Ok(opcode_60),
			0x61 => Ok(opcode_61),
			0x62 => Ok(opcode_62),
			0x63 => Ok(opcode_63),
			0x64 => Ok(opcode_64),
			0x65 => Ok(opcode_65),
			0x66 => Ok(opcode_66),
			0x67 => Ok(opcode_67),
			0x68 => Ok(opcode_68),
			0x69 => Ok(opcode_69),
			0x6a => Ok(opcode_6a),
			0x6b => Ok(opcode_6b),
			0x6c => Ok(opcode_6c),
			0x6d => Ok(opcode_6d),
			0x6e => Ok(opcode_6e),
			0x6f => Ok(opcode_6f),
			0x70 => Ok(opcode_70),
			0x71 => Ok(opcode_71),
			0x72 => Ok(opcode_72),
			0x73 => Ok(opcode_73),
			0x74 => Ok(opcode_74),
			0x75 => Ok(opcode_75),
			0x77 => Ok(opcode_77),
			0x78 => Ok(opcode_78),
			0x79 => Ok(opcode_79),
			0x7a => Ok(opcode_7a),
			0x7b => Ok(opcode_7b),
			0x7c => Ok(opcode_7c),
			0x7d => Ok(opcode_7d),
			0x7e => Ok(opcode_7e),
			0x7f => Ok(opcode_7f),
			0xc1 => Ok(opcode_c1),
			0xc5 => Ok(opcode_c5),
			0xd1 => Ok(opcode_d1),
			0xd5 => Ok(opcode_d5),
			0xe0 => Ok(opcode_e0),
			0xe1 => Ok(opcode_e1),
			0xe2 => Ok(opcode_e2),
			0xe5 => Ok(opcode_e5),
			0xea => Ok(opcode_ea),
			0xf0 => Ok(opcode_f0),
			0xf1 => Ok(opcode_f1),
			0xf2 => Ok(opcode_f2),
			0xf5 => Ok(opcode_f5),
			0xf8 => Ok(opcode_f8),
			0xf9 => Ok(opcode_f9),
			0xfa => Ok(opcode_fa),
			0xcb => {
				let next_byte = self.fetch()?;
				self.decode_cb(next_byte)
			},
			_ => Err(GameboyError::BadOpcode(opcode))
		}
	}

	/// Decode a 16-bit opcode that starts with 0xCB.
	pub fn decode_cb(&self, opcode: u8) -> Result<Instruction, GameboyError> {
		match opcode {
			// TODO add all CB opcodes here!
			_ => Err(GameboyError::BadOpcode(opcode))
		}
	}

}
