// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Emulator hardware emulation configuration and preferences.

/// The hardware specification for the different models differ.
pub enum HardwareModel {
	/// Original GameBoy
	GB,
	/// Gameboy Color
	GBC,
	/// GameBoy Pocket (not intended to be supported soon)
	GBP,
	/// Super GameBoy (not intended to be supported soon)
	SGB,
}

/// Emulation settings and preferences goes here.
pub struct Config {
	/// The model of the emulated machine
	pub model: HardwareModel,
}

impl Default for Config {
	fn default() -> Self {
		Config {
			model: HardwareModel::GB
		}
	}
}
