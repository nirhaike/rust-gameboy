// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! An example implementation of an emulator using the gameboy core library.

extern crate minifb;

use std::fs;
use std::env;
use std::fmt;
use std::vec::Vec;
use std::thread::sleep;
use std::time::Duration;

use minifb::{Key, Window, WindowOptions};

use gameboy_core::cpu::*;
use gameboy_core::bus::joypad;
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

// Maps minifb keys to emulator keys.
fn map_input_key(key: &Key) -> joypad::Key {
	match key {
		Key::Right => joypad::Key::Right,
		Key::Left => joypad::Key::Left,
		Key::Down => joypad::Key::Down,
		Key::Up => joypad::Key::Up,
		Key::Z => joypad::Key::A,
		Key::X => joypad::Key::B,
		Key::Space => joypad::Key::Select,
		Key::Enter => joypad::Key::Start,
		_ => panic!("Received an unexpected key.")
	}
}

fn update_key_state(cpu: &mut Cpu, window: &Window) {
	for key in [Key::Right, Key::Left, Key::Down, Key::Up, Key::Z, Key::X, Key::Space, Key::Enter].iter() {
		let emulator_key = map_input_key(key);
		let key_down: bool = window.is_key_down(*key);

		if key_down {
			cpu.with_controller(|joypad| joypad.down(emulator_key))
		} else {
			cpu.with_controller(|joypad| joypad.up(emulator_key))
		}
	}
}

fn main() -> Result<(), EmulatorError> {
	// Initialize the frame buffer
	let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

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
	let mut total: usize = 0;

	while window.is_open() && !window.is_key_down(Key::Escape) {
		match cpu.execute() {
			Ok(elapsed) => { cycles += elapsed; total += elapsed; }
			Err(err) => { 
				println!("Total cycles: {:?}", total);
				return Err(err.into());
			}
		}

		// Update the frame buffer every now and then..
		// TODO change this to an actual precise time-based approach!
		if cycles > 100000 {
			cpu.flush(&mut buffer);
			window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
			
			update_key_state(&mut cpu, &window);

			cycles -= 100000;
			sleep(Duration::from_millis(8));

		}
	}

	Ok(())
}
