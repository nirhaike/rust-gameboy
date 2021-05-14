// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This library provides emulation of the gameboy's Z80-like CPU and it's peripherals,
//! as described in the publicly available "Game Boy CPU Manual".

#[cfg(test)]
#[macro_use]
extern crate std;

pub mod bus;
pub mod cpu;
pub mod state;
pub mod config;

use core::fmt;

/// The library's exported errors.
pub enum GameboyError {
	/// Cartridge loading error.
	Cartridge(&'static str),
	/// IO related error.
	Io(&'static str),
}

impl fmt::Display for GameboyError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
            GameboyError::Cartridge(ref info) => write!(f, "Cartridge error: {}", info),
            GameboyError::Io(ref info) => write!(f, "IO error: {}", info),
        }
	}
}
