// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy cpu's arithmetic and logic unit.

use super::Cpu;

/// Implementation of 8-bit arithmetic operations.
pub mod alu8 {
	use super::*;

	/// Adds the given arguments, sets the relevant flags accordinately
	/// and return the result.
	pub fn add(_cpu: &mut Cpu, lhs: u8, rhs: u8) -> u8 {
		// TODO complete this!
		((lhs as u16 + rhs as u16) & 0xff) as u8
	}
}
