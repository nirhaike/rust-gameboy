// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulate the real time clock, that appears in type-3 MBCs.

use super::Memory;
use crate::GameboyError;
use core::ops::RangeInclusive;

/// The rtc registers are mapped to 0xA000-0xBF00 whenever
/// a value within the control range is written to the RAM/RTC select
/// register.
pub const RTC_CONTROL_RANGE: RangeInclusive<u8> = 0x8..=0xC;

/// The cartridge's real-time clock registers.
///
/// Internally, the clock is incremented using an internal counter,
/// and the registers are updated whenever the clock data is latched
/// by the software.
#[allow(dead_code)]
pub struct Rtc {
	registers: [u8; 5],
	active_register: u8,
	counter: usize,
}

enum RtcRegister {
	Seconds = 0,
	Minutes = 1,
	Hours = 2,
	DaysLow = 3,
	Flags = 4,
}

impl Rtc {
	/// Create a new real-time clock.
	pub fn new() -> Self {
		Rtc {
			registers: [0_u8; 5],
			active_register: 0,
			counter: 0,
		}
	}

	/// Returns the register containing the seconds counter.
	pub fn seconds(&self) -> u8 {
		self.registers[RtcRegister::Seconds as usize]
	}

	/// Returns the register containing the minutes counter.
	pub fn minutes(&self) -> u8 {
		self.registers[RtcRegister::Minutes as usize]
	}

	/// Returns the register containing the hours counter.
	pub fn hours(&self) -> u8 {
		self.registers[RtcRegister::Hours as usize]
	}

	/// The days are represented by 9 bits.
	/// This function returns the lower 8 bits.
	pub fn days_low(&self) -> u8 {
		self.registers[RtcRegister::DaysLow as usize]
	}

	/// The flags register contains flags and another days bit:
	///
	/// * Bit 0 - Day counter's MSB.
	/// * Bit 6 - Halt flag (0 = Active, 1 = Stop timer).
	/// * Bit 7 - Day counter carry flag.
	pub fn flags(&self) -> u8 {
		self.registers[RtcRegister::Flags as usize]
	}

	/// Increment the clock.
	pub fn tick(&self) {
		unimplemented!();
	}

	/// Fetch the clock data into the rtc's registers.
	///
	/// The latching process consists of writing 0x00 and then 0x01 to
	/// the Latch Clock Data register.
	pub fn latch(&mut self, _value: u8) {
		unimplemented!();
	}

	/// Set the currently memory mapped RTC register.
	pub fn set_active_register(&mut self, value: u8) -> Result<(), GameboyError> {

		if RTC_CONTROL_RANGE.contains(&value) {
			self.active_register = value - 0x08;
			return Ok(())
		}

		Err(GameboyError::BadValue(value))
	}
}

impl Memory for Rtc {
	/// Writes to the rtc's currently active register.
	fn write(&mut self, _address: u16, _value: u8) -> Result<(), GameboyError> {
		unimplemented!();
	}

	/// Reads the rtc's currently active register.
	fn read(&self, _address: u16) -> Result<u8, GameboyError> {
		unimplemented!();
	}
}
