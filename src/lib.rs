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
pub mod config;
