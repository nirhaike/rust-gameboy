// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Abstraction for the cpu's interrupts.

use core::iter::Iterator;

/// Marks which interrupts are currently active.
pub type InterruptMask = u8;

/// Represents a peripheral that may raise interrupts.
pub trait InterruptSource {
	/// Returns the active interrupts mask.
	fn interrupts(&self) -> InterruptMask;

	/// Clears the peripheral's interrupt mask.
	fn clear(&mut self);
}

/// Interrupts that can be thrown by peripherals.
pub enum Interrupt {
	/// Triggered when the LCD controller enters V-Blank at scanline 144.
	VerticalBlank,
	/// Triggered by configured LCD events (such as scanline coincidence).
	LcdStat,
	/// Triggered when TIMA overflows, with a delay of a single cycle.
	Timer,
	/// Triggered when a serial transfer of 1 byte is complete.
	Serial,
	/// Triggered when one of the P1 input lines is changed from 1 to 0.
	Joypad,
}

impl Interrupt {
	/// Get the identifier of the given interrupt.
	pub fn ordinal(&self) -> u8 {
		match self {
			Interrupt::VerticalBlank => 0,
			Interrupt::LcdStat => 1,
			Interrupt::Timer => 2,
			Interrupt::Serial => 3,
			Interrupt::Joypad => 4,
		}
	}

	/// Get the relevant bit of the given interrupt.
	pub fn value(&self) -> u8 {
		1 << self.ordinal()
	}
}

/// Iterates over interrupts that the Ppu has raised.
pub struct InterruptIter {
	/// The iterator's active interrupts mask.
	/// Iterated interrupts are popped from the mask.
	pub mask: InterruptMask,
}

impl InterruptIter {
	/// Create a new interrupt iterator.
	pub fn new(mask: InterruptMask) -> Self {
		InterruptIter {
			mask
		}
	}
}

impl Iterator for InterruptIter {
	type Item = Interrupt;

	fn next(&mut self) -> Option<Self::Item> {
		// V-Blank interrupt
		if self.mask & 1 != 0 {
			self.mask &= !1;
			Some(Interrupt::VerticalBlank)
		// LCD Status interrupt
		} else if self.mask & 2 != 0 {
			self.mask &= !2;
			Some(Interrupt::LcdStat)
		} else if self.mask & 4 != 0 {
			self.mask &= !4;
			Some(Interrupt::Timer)
		} else if self.mask & 8 != 0 {
			self.mask &= !8;
			Some(Interrupt::Serial)
		} else if self.mask & 16 != 0 {
			self.mask &= !16;
			Some(Interrupt::Joypad)
		} else {
			None
		}
	}

}