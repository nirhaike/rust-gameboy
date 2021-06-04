// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Emulate I/O memory-mapped registers.

use super::Memory;
use super::consts::*;
use super::memory_range::*;

use crate::config::*;
use crate::GameboyError;

#[allow(unused)]
mod consts {
	use super::*;

	/// The total size of the registers' memory mapping.
	pub const IO_SIZE: usize = 0x4C;

	pub const IO_P1: u16 = 0xFF00;
	pub const IO_SB: u16 = 0xFF01;
	pub const IO_SC: u16 = 0xFF02;
	pub const IO_DIV: u16 = 0xFF04;
	pub const IO_TIMA: u16 = 0xFF05;
	pub const IO_TMA: u16 = 0xFF06;
	pub const IO_TAC: u16 = 0xFF07;
	pub const IO_IF: u16 = 0xFF0F;
	pub const IO_NR10: u16 = 0xFF10;
	pub const IO_NR11: u16 = 0xFF11;
	pub const IO_NR12: u16 = 0xFF12;
	pub const IO_NR13: u16 = 0xFF13;
	pub const IO_NR14: u16 = 0xFF14;
	pub const IO_NR21: u16 = 0xFF16;
	pub const IO_NR22: u16 = 0xFF17;
	pub const IO_NR23: u16 = 0xFF18;
	pub const IO_NR24: u16 = 0xFF19;
	pub const IO_NR30: u16 = 0xFF1A;
	pub const IO_NR31: u16 = 0xFF1B;
	pub const IO_NR32: u16 = 0xFF1C;
	pub const IO_NR33: u16 = 0xFF1D;
	pub const IO_NR34: u16 = 0xFF1E;
	pub const IO_NR41: u16 = 0xFF20;
	pub const IO_NR42: u16 = 0xFF21;
	pub const IO_NR43: u16 = 0xFF22;
	pub const IO_NR44: u16 = 0xFF23;
	pub const IO_NR50: u16 = 0xFF24;
	pub const IO_NR51: u16 = 0xFF25;
	pub const IO_NR52: u16 = 0xFF26;
	pub const IO_WAVE_PATTERN: MemoryRange = make_range!(0xFF30, 0xFF3F);
	pub const IO_LCDC: u16 = 0xFF40;
	pub const IO_STAT: u16 = 0xFF41;
	pub const IO_SCY: u16 = 0xFF42;
	pub const IO_SCX: u16 = 0xFF43;
	pub const IO_LY: u16 = 0xFF44;
	pub const IO_LYC: u16 = 0xFF45;
	pub const IO_DMA: u16 = 0xFF46;
	pub const IO_BGP: u16 = 0xFF47;
	pub const IO_OBP0: u16 = 0xFF48;
	pub const IO_OBP1: u16 = 0xFF49;
	pub const IO_WY: u16 = 0xFF4A;
	pub const IO_WX: u16 = 0xFF4B;
}

/// Convert address constants to register array offset.
macro_rules! port_offset {
	($address:tt) => (($address - 0xFF00) as usize)
}

use consts::*;

/// Handles read and write operation on I/O registers.
pub struct IOPorts {
	/// Registers that are mapped to the range 0xFF00-0xFF4B.
	registers: [u8; IO_SIZE],
	/// Interrupt enable (0xFFFF).
	ie: u8,
}

impl IOPorts {
	/// Initialize the I/O registers with boot state.
	pub fn new(config: &Config) -> Self {
		let mut io = IOPorts {
			registers: [0_u8; IO_SIZE],
			ie: 0,
		};

		// Reset the registers' state.
		io.reset(config);

		io
	}

	/// Reset the I/O registers.
	pub fn reset(&mut self, config: &Config) {
		self.registers[port_offset!(IO_NR10)] = 0x80;
		self.registers[port_offset!(IO_TMA)] = 0x00;
		self.registers[port_offset!(IO_TAC)] = 0x00;
		self.registers[port_offset!(IO_NR10)] = 0x80;
		self.registers[port_offset!(IO_NR11)] = 0xBF;
		self.registers[port_offset!(IO_NR12)] = 0xF3;
		self.registers[port_offset!(IO_NR14)] = 0xBF;
		self.registers[port_offset!(IO_NR21)] = 0x3F;
		self.registers[port_offset!(IO_NR22)] = 0x00;
		self.registers[port_offset!(IO_NR24)] = 0xBF;
		self.registers[port_offset!(IO_NR30)] = 0x7F;
		self.registers[port_offset!(IO_NR31)] = 0xFF;
		self.registers[port_offset!(IO_NR32)] = 0x9F;
		self.registers[port_offset!(IO_NR34)] = 0xBF;
		self.registers[port_offset!(IO_NR41)] = 0xFF;
		self.registers[port_offset!(IO_NR42)] = 0x00;
		self.registers[port_offset!(IO_NR43)] = 0x00;
		self.registers[port_offset!(IO_NR44)] = 0xBF;
		self.registers[port_offset!(IO_NR50)] = 0x77;
		self.registers[port_offset!(IO_NR51)] = 0xF3;
		self.registers[port_offset!(IO_NR52)] = match config.model {
			HardwareModel::SGB => 0xF0,
			_ => 0xF1,
		};
		self.registers[port_offset!(IO_LCDC)] = 0x91;
		self.registers[port_offset!(IO_SCY)] = 0x00;
		self.registers[port_offset!(IO_SCX)] = 0x00;
		self.registers[port_offset!(IO_LYC)] = 0x00;
		self.registers[port_offset!(IO_BGP)] = 0xFC;
		self.registers[port_offset!(IO_OBP0)] = 0xFF;
		self.registers[port_offset!(IO_OBP1)] = 0xFF;
		self.registers[port_offset!(IO_WY)] = 0x00;
		self.registers[port_offset!(IO_WX)] = 0x00;
		self.ie = 0x00;
	}
}

impl Memory for IOPorts {
	fn write(&mut self, _address: u16, _value: u8) -> Result<(), GameboyError> {
		unimplemented!();
	}

	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		match address {

			// Specific behaviors will be added here.
			memory_range!(MMAP_IO_PORTS) => {
				let offset = (address as usize) - range_start!(MMAP_IO_PORTS);
				Ok(self.registers[offset])
			}
			memory_range!(MMAP_INTERRUPT_EN) => {
				Ok(self.ie)
			}
			_ => {
				Err(GameboyError::BadAddress(address))
			}
		}
	}
}
