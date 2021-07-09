// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This library provides emulation of the gameboy's Z80-like CPU and it's peripherals,
//! as described in the publicly available "Game Boy CPU Manual".

#[cfg(any(test, feature = "debug"))]
#[macro_use]
extern crate std;
extern crate core;
// The alloc crate is optional, and used for allocating the cartridge controller's
// ram on the heap.
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bus;
pub mod cpu;
pub mod config;

use core::fmt;

/// The library's exported errors.
pub enum GameboyError {
	/// Unimplemented feature error.
	NotImplemented,
	/// Cartridge operation error.
	Cartridge(&'static str),
	/// Generic IO related error.
	Io(&'static str),
	/// Unexpected address error.
	BadAddress(u16),
	/// Invalid opcode error.
	BadOpcode(u8),
	/// Invalid value written to a register.
	BadValue(u8),
}

impl fmt::Display for GameboyError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			GameboyError::NotImplemented => write!(f, "Not implemented"),
            GameboyError::Cartridge(ref info) => write!(f, "Cartridge error: {}", info),
            GameboyError::Io(ref info) => write!(f, "IO error: {}", info),
            GameboyError::BadAddress(address) => write!(f, "Bad address: 0x{:x}", address),
            GameboyError::BadOpcode(value) => write!(f, "Bad opcode: 0x{:x}", value),
            GameboyError::BadValue(value) => write!(f, "Bad value: {}", value),
        }
	}
}

impl fmt::Debug for GameboyError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		(self as &dyn fmt::Display).fmt(f)
	}
}
