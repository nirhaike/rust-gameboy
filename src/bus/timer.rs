// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
//! Gameboy's timer controller.

use super::Memory;
use super::memory_range::*;

use crate::GameboyError;

use crate::config::*;
use crate::cpu::interrupts::*;

pub mod consts {
	use super::*;

	pub const IO_DIV: u16 = 0xFF04;
	pub const IO_TIMA: u16 = 0xFF05;
	pub const IO_TMA: u16 = 0xFF06;
	pub const IO_TAC: u16 = 0xFF07;

	pub const MMAP_IO_TIMER: MemoryRange = make_range!(0xFF04, 0xFF07);
}

use consts::*;

pub struct Timer {
	/// DIV consists of 2 bytes, and only the higher 8 bits are exposed to the cpu.
	div: u16,
	/// Timer counter.
	tima: u8,
	/// Timer modulo.
	tma: u8,
	/// Timer control.
	tac: Tac,

	interrupt_flag: InterruptMask,
}

struct Tac {
	pub enable: bool,
	pub frequency: u8,
}

impl Timer {
	/// Initialize a new timer instance.
	pub fn new(config: &Config) -> Self {
		let mut timer = Timer {
			div: 0,
			tima: 0,
			tma: 0,
			tac: Tac::new(),
			interrupt_flag: 0,
		};

		timer.reset(config);

		timer
	}

	/// Reset the peripheral to boot state.
	pub fn reset(&mut self, config: &Config) {
		match config.model {
			HardwareModel::GB | HardwareModel::SGB => {
				self.div = 0xabcc;
			}
			HardwareModel::GBC => {
				// TODO div's value depends on whether it is a GBC or GB game.
				self.div = 0x1ea0;
			}
			HardwareModel::GBP => {
				self.div = 0x1ea4;
			}
		}

		self.tima = 0;
		self.tma = 0;
		self.tac.reset();
	}

	/// Update the timer's state according to the elapsed time.
	pub fn process(&mut self, cycles: usize) {
		let new_div = self.div.wrapping_add(cycles as u16);

		// Get the timer's frequency from the control register.
		let div_bit = [512, 8, 32, 128][self.tac.frequency as usize];
		
		if self.tac.enable && (self.div & div_bit) != (new_div & div_bit) {
			// Increment the timer.
			self.tima = self.tima.wrapping_add(1);

			if self.tima == 0 {
				self.interrupt_flag |= Interrupt::Timer.value();
				self.tima = self.tma;
			}
		}

		self.div = new_div;
	}
}

impl Memory for Timer {
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		match address {
			IO_DIV => {
				// div is set to 0 on write.
				self.div = 0;
			}
			IO_TIMA => {
				self.tima = value;
			}
			IO_TMA => {
				self.tma = value;
			}
			IO_TAC => {
				self.tac.write(value);
			}
			_ => {
				panic!("Write operation is not implemented for {:x}", address);
			}
		}

		Ok(())
	}

	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		match address {
			IO_DIV => {
				// div is set to 0 on write.
				Ok(((self.div & 0xFF00) >> 8) as u8)
			}
			IO_TIMA => {
				Ok(self.tima)
			}
			IO_TMA => {
				Ok(self.tma)
			}
			IO_TAC => {
				Ok(self.tac.read())
			}
			_ => {
				panic!("Read operation is not implemented for {:x}", address);
			}
		}
	}
}

impl InterruptSource for Timer {
	fn interrupts(&self) -> InterruptMask {
		self.interrupt_flag
	}

	fn clear(&mut self) {
		self.interrupt_flag = 0;
	}
}

#[allow(unused)]
impl Tac {
	pub fn new() -> Self {
		Tac { enable: false, frequency: 0 }
	}

	pub fn reset(&mut self) {
		self.enable = false;
		self.frequency = 0;
	}

	pub fn write(&mut self, value: u8) {
		self.enable = (value & 4) != 0;
		self.frequency = value & 3;
	}

	pub fn read(&self) -> u8 {
		self.frequency + if self.enable { 4 } else { 0 }
	}
}
