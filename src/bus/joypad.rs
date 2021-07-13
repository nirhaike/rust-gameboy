// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
//! Gameboy's joypad controller.

use super::Memory;

use crate::GameboyError;
use crate::cpu::interrupts::*;

pub mod consts {
	use super::*;

	pub const IO_P1: u16 = 0xFF00;
}

use consts::*;

/// The matrix layout for the P1 register, according to the Gameboy CPU manual.
pub enum Key {
	Right,
	Left,
	Up,
	Down,
	A,
	B,
	Select,
	Start,
}

impl Key {
	pub fn value(&self) -> u8 {
		match self {
			Key::Right => 1,
			Key::Left => 2,
			Key::Up => 4,
			Key::Down => 8,
			Key::A => 16,
			Key::B => 32,
			Key::Select => 64,
			Key::Start => 128,
		}
	}
}

pub trait Controller {
	/// Mark the given key as currently pressed.
	fn down(&mut self, key: Key);

	/// Mark the given key as released.
	fn up(&mut self, key: Key);
}

pub struct Joypad {
	data: u8,
	/// If true, P15 out port is being selected, otherwise P14 is used.
	select: u8,
	interrupt_flag: InterruptMask,
}


impl Joypad {
	/// Initialize a new timer instance.
	pub fn new() -> Self {
		Joypad {
			data: 0,
			select: 0,
			interrupt_flag: 0,
		}
	}

	/// Update the joypad's state according to the elapsed time.
	pub fn process(&mut self, _cycles: usize) {}
}

impl Controller for Joypad {
	fn down(&mut self, key: Key) {
		self.data &= !key.value();
		self.interrupt_flag |= Interrupt::Joypad.value();
	}

	fn up(&mut self, key: Key) {
		self.data |= key.value();
	}
}

impl InterruptSource for Joypad {
	fn interrupts(&self) -> InterruptMask {
		self.interrupt_flag
	}

	fn clear(&mut self) {
		self.interrupt_flag = 0;
	}
}

impl Memory for Joypad {
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		assert!(address == IO_P1);

		self.select = value;

		Ok(())
	}

	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		assert!(address == IO_P1);

		if self.select & 0x20 == 0 {
			Ok(self.select | ((self.data >> 4) & 0xf))
		} else if self.select & 0x10 == 0 {
			Ok(self.select | (self.data & 0xf))
		} else {
			Ok(self.select)
		}
	}
}
