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
			0x06 => Ok(opcode_06),
			0x0e => Ok(opcode_0e),
			0xcb => {
				let next_byte = self.fetch()?;
				self.decode_cb(next_byte)
			},
			// TODO add all opcodes here!
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
