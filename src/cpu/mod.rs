// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Gameboy's processor emulation.

pub mod alu;
pub mod state;
pub mod decode;
pub mod interrupts;
pub mod disassemble;
pub mod instructions;

use num::PrimInt;
use core::mem::size_of;
use core::ops::{AddAssign, Shl};

use state::*;
use state::registers::*;
use instructions::{Instruction, enter_interrupt};

use crate::GameboyError;
use crate::config::Config;
use crate::bus::joypad::Controller;

use crate::bus::*;
use crate::bus::cartridge::*;
use crate::cpu::interrupts::*;

/// The gameboy's processor.
///
/// This struct contains the complete emulator's state.
#[allow(dead_code)]
pub struct Cpu<'a> {
	// Interrupts, system tick, cpu speed, serial ports and etc. should come here

	/// The cpu's registers.
	registers: CpuState<'a>,
	/// The devices' memory mapping
	pub mmap: SystemBus<'a>,
	/// The emulator's configuration
	pub config: &'a Config,

	/// TODO
	pub halting: bool,
	/// If we halt the cpu when interrupts are disabled, the original cpu had a bug
	/// in which it fetches the byte after the halt twice.
	halt_bug: bool,
	/// The processor has a delay of a single instruction after EI before actually
	/// enabling interrupts.
	ime_delay: bool,
	/// TODO remove this.
	countdown: usize,
}

impl<'a> Cpu<'a> {
	/// Initializes a new virtual cpu
	#[inline(always)]
	pub fn new(config: &'a Config, cartridge: &'a mut Cartridge<'a>) -> Self {
		Cpu {
			registers: CpuState::new(config),
			mmap: SystemBus::new(&config, cartridge),
			config,
			halting: false,
			halt_bug: false,
			ime_delay: false,
			countdown: 0,
		}
	}

	/// Halt the cpu.
	pub fn halt(&mut self) {
		self.halting = true;

		if !self.registers.ime() {
			self.halt_bug = true;
		}
	}

	/// Enable interrupts with a delay of a single instruction.
	pub fn toggle_ime_delayed(&mut self) {
		self.ime_delay = true;
	}

	/// Apply the given closure to the game controller.
	pub fn with_controller<F>(&mut self, closure: F)
		where F: FnOnce(&mut dyn Controller) -> () {
			closure(&mut self.mmap.joypad);
	}

	/// Reads the next instruction bytes and increments the program counter appropriately.
	///
	/// The function works in little-endian, that is, when reading 2 bytes,
	/// the first byte will be the least-significant one.
	pub fn fetch<T: PrimInt + AddAssign + Shl<Output=T>>(&mut self) -> Result<T, GameboyError> {
		let mut result: T = num::cast(0).unwrap();

		for i in 0..size_of::<T>() {
			// Read the next byte.
			let pc: u16 = self.registers.get(Register::PC);
			let data: T = num::cast::<u8, T>(self.mmap.read(pc)?).unwrap();

			// We're using little-endianity.
			result += data << num::cast::<usize, T>(8 * i).unwrap();

			if self.halt_bug {
				// The halt bug prevents the program counter from being incremented once.
				self.halt_bug = false;
			} else {
				// Move the PC forward.
				self.registers.set(Register::PC, pc + 1);
			}
		}

		Ok(result)
	}

	/// Writes the display's data to the given frame buffer.
	pub fn flush(&mut self, frame_buffer: &mut [u32]) {
		self.mmap.ppu.flush(frame_buffer);
	}

	/// Emulates the execution of a single instruction.
	///	This function also processes the peripherals and enters interrupts if any.
	///
	/// Returns the number of clock cycles the instruction has taken.
	pub fn execute(&mut self) -> Result<usize, GameboyError> {
		// Enter an interrupt if any (and if interrupts are enabled).
		let mut num_cycles = self.handle_interrupts()?;

		if !self.halting {
			num_cycles += self.execute_single()?;
		} else {
			num_cycles += 4;
		}

		// Enable interrupts if needed
		if self.ime_delay {
			self.registers.set_ime(true);
		}

		// Progress the peripherals.
		self.mmap.process(num_cycles);

		Ok(num_cycles)
	}

	/// Emulates the execution of a single instruction.
	///
	/// Returns the number of clock cycles the instruction has taken.
	pub fn execute_single(&mut self) -> Result<usize, GameboyError> {
		let _address: u16 = self.registers.get(Register::PC);

		if _address == 0x7db8 && self.countdown == 0 {
			self.countdown = 2000;
		}

		if self.countdown > 0 {
			self.countdown -= 1;
			if self.countdown == 1 {
				// panic!("SHET");
			}
		}

		// Fetch the opcode from the memory.
		let opcode: u8 = self.fetch()?;

		// TODO remove this!
		#[cfg(feature = "debug")]
		{
			println!("0x{:04x}: ({:02x}) {}", _address, opcode, disassemble::disassemble(self, _address)?);
			if opcode == 0xcd {
				println!("Branch target: {:02x} {:02x}", self.mmap.read(_address + 1)?, self.mmap.read(_address + 2)?);
			}
		}

		// Decode the given opcode.
		let insn: Instruction = self.decode(opcode)?;

		// Execute and return the number of cycles taken.
		Ok(insn(self)?)
	}

	fn handle_interrupts(&mut self) -> Result<usize, GameboyError> {
		if !self.registers.ime() {
			// Stop halting if there's any active interrupt.
			// We wake the cpu in a case of an interrupt, but we won't
			// enter the ISR if interrupts are disabled.
			if self.halting && self.mmap.interrupt_flag != 0 {
				self.halting = false;
			}
			return Ok(0);
		}

		if let Some(interrupt) = self.mmap.fetch_interrupt() {
			// Stop halting (if relevant) and enter the ISR.
			self.halting = false;

			let isr = match interrupt {
				Interrupt::VerticalBlank => 0x0040,
				Interrupt::LcdStat => 0x0048,
				Interrupt::Timer => 0x0050,
				Interrupt::Serial => 0x0058,
				Interrupt::Joypad => 0x0060,
			};

			return Ok(enter_interrupt(self, isr)?);
		}

		Ok(0)
	}
}

#[cfg(test)]
#[cfg(feature = "alloc")]
pub mod tests {
	use super::*;
	use alloc::boxed::Box;

	/// With-closure for running logic with an initialized cpu instance.
	pub fn with_cpu<F>(callback: F) -> Result<(), GameboyError>
		where F: FnOnce(&mut Cpu) -> Result<(), GameboyError> {
		// Initialize the cpu
		let config = Config::default();
		let mut rom = cartridge::tests::empty_rom(CartridgeType::MBC3);
		let mut ram: Box<[u8]> = Cartridge::make_ram(&rom)?;
		let mut cartridge = Cartridge::new(&mut rom, &mut ram)?;

		let mut cpu = Cpu::new(&config, &mut cartridge);

		callback(&mut cpu)
	}

	#[test]
	fn test_fetch() -> Result<(), GameboyError> {
		with_cpu(|cpu| {
			// Move the program counter to the RAM bank.
			cpu.registers.set(Register::PC, 0xA000);

			// Write arbitrary data to the memory starting from the program counter.
			let data: &[u8] = &[1, 2, 3];
			cpu.mmap.cartridge.set_ram_enabled(true);
			cpu.mmap.write_all(cpu.registers.get(Register::PC), data)?;

			// Make sure that fetch works as expected.
			assert!(cpu.fetch::<u16>()? == 0x0201);
			assert!(cpu.fetch::<u8>()? == 0x03);

			Ok(())
		})
	}
}
