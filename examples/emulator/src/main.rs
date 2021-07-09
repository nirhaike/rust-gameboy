// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! An example implementation of an emulator using the gameboy core library.

extern crate gameboy_core;

use std::fs;
use std::env;
use std::fmt;
use std::vec::Vec;

use minifb::{Key, Window, WindowOptions};

use gameboy_core::cpu::*;
use gameboy_core::GameboyError;
use gameboy_core::config::Config;
use gameboy_core::bus::cartridge::*;

const WIDTH: usize = 160;
const HEIGHT: usize = 144;

enum EmulatorError {
    Std(std::io::Error),
    Gameboy(GameboyError),
}

impl From<std::io::Error> for EmulatorError {
    fn from(e: std::io::Error) -> Self {
        EmulatorError::Std(e)
    }
}

impl From<GameboyError> for EmulatorError {
    fn from(e: GameboyError) -> Self {
        EmulatorError::Gameboy(e)
    }
}

impl fmt::Debug for EmulatorError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			EmulatorError::Std(ref err) => err.fmt(f),
            EmulatorError::Gameboy(ref err) => err.fmt(f),
        }
	}
}

fn main() -> Result<(), EmulatorError> {
	// Initialize the frame buffer
	let buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

	let mut window = Window::new(
        "Gameboy",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    ).unwrap_or_else(|e| { panic!("{}", e); });

	// Initialize the cpu.
	let config = Config::default();

	// Load the cartridge.
	let args: Vec<String> = env::args().collect();
	let rom_fname = &args[1];
	let mut rom: Box<[u8]> = fs::read(rom_fname)?.into();
	let mut ram: Box<[u8]> = Cartridge::make_ram(&rom)?;
	let mut cartridge = Cartridge::new(&mut rom, &mut ram)?;

	let mut cpu = Cpu::new(&config, &mut cartridge);

	// Start executing.
	let mut cycles: usize = 0;

	while window.is_open() && !window.is_key_down(Key::Escape) {
		match cpu.execute() {
			Ok(elapsed) => { cycles += elapsed; }
			Err(err) => { 
				println!("Total cycles: {:?}", cycles);
				return Err(err.into());
			}
		}

		// Update the frame buffer every now and then
		// TODO change this to an actual time-based approach!
		if cycles > 0x10000 {
			window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
			cycles -= 0x10000;
		}
	}

	Ok(())
}
