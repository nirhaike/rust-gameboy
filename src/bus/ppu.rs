// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Gameboy's lcd controller / picture processing unit.

use super::Memory;
use super::consts::*;
use super::memory_range::*;

use crate::GameboyError;
use crate::cpu::interrupts::*;

#[allow(unused, missing_docs)]
pub mod consts {
	use super::*;

	// Position and scrolling
	pub const IO_LCDC: u16 = 0xFF40;
	pub const IO_STAT: u16 = 0xFF41;
	pub const IO_SCY: u16 = 0xFF42;
	pub const IO_SCX: u16 = 0xFF43;
	pub const IO_LY: u16 = 0xFF44;
	pub const IO_LYC: u16 = 0xFF45;
	pub const IO_BGP: u16 = 0xFF47;
	pub const IO_OBP0: u16 = 0xFF48;
	pub const IO_OBP1: u16 = 0xFF49;
	pub const IO_WY: u16 = 0xFF4A;
	pub const IO_WX: u16 = 0xFF4B;

	// 0xFF46 is in the range although it's unrelated, but it appears on a higher
	// match arm on the system bus so it won't reach our I/O handlers.
	pub const MMAP_IO_DISPLAY: MemoryRange = make_range!(0xFF40, 0xFF4B);

	// Color palettes (GBC)
	pub const IO_BGPI: u16 = 0xFF68;
	pub const IO_BGPD: u16 = 0xFF69;
	pub const IO_OBPI: u16 = 0xFF6A;
	pub const IO_OBPD: u16 = 0xFF6B;

	pub const MMAP_IO_PALETTES: MemoryRange = make_range!(0xFF68, 0xFF6B);

	pub const VRAM_SIZE: usize = 0x2000;
	pub const OAM_SIZE: usize = 0x100;

	pub const WIDTH: usize = 160;
	pub const HEIGHT: usize = 144;

	pub const PALETTE: [Color; 4] = [
		0x081820,
		0x346856,
		0x88c070,
		0xe0f8d0,
	];
}

use consts::*;

/// Represents a single color within a palette.
type Color = u32;

/// The lcd controller peripheral has four states, and 154 cycles between
/// these states corresponds to a single frame when the LCD is on.
#[derive(Clone, Copy, PartialEq)]
#[allow(missing_docs)]
pub enum PpuMode {
	Hblank,
	Vblank,
	SearchOam,
	RenderLine,
}

/// The gameboy's lcd controller.
#[allow(unused)]
pub struct Ppu {
	buffer: [Color; WIDTH * HEIGHT],
	vram: [u8; VRAM_SIZE],
	oam: [u8; OAM_SIZE],
	// TODO add canvas: Option<&mut [u8; ??]>

	lcdc: Lcdc,
	stat: Stat,
	scy: u8,
	scx: u8,
	ly: u8,
	lyc: u8,
	bgp: u8,
	obp0: u8,
	obp1: u8,
	wy: u8,
	wx: u8,

	mode: PpuMode,
	mode_counter: usize,
	interrupt_flag: InterruptMask,
}

struct Lcdc {
	data: u8,
}

struct Stat {
	// Consists of bits 2-6 (RW).
	data: u8,
	// Consists of bit 2 (RO).
	signal: u8,
	// Consists of bits 0-1 (RO).
	mode: u8,
}

impl Ppu {
	/// Initialize a new ppu instance.
	pub fn new() -> Self {
		let mut ppu = Ppu {
			buffer: [0; WIDTH * HEIGHT],
			vram: [0; VRAM_SIZE],
			oam: [0; OAM_SIZE],
			lcdc: Lcdc::new(),
			stat: Stat::new(),
			scy: 0,
			scx: 0,
			ly: 0,
			lyc: 0,
			bgp: 0,
			obp0: 0,
			obp1: 0,
			wy: 0,
			wx: 0,
			mode: PpuMode::SearchOam,
			mode_counter: 0,
			interrupt_flag: 0,
		};

		ppu.reset();

		ppu
	}

	/// Reset this peripheral to boot state.
	pub fn reset(&mut self) {
		self.mode = PpuMode::SearchOam;
		self.lcdc.reset();
		self.stat.reset();
		self.stat.set_mode(self.mode);
		self.scy = 0x00;
		self.scx = 0x00;
		self.lyc = 0x00;
		self.bgp = 0xFC;
		self.obp0 = 0xFF;
		self.obp1 = 0xFF;
		self.wy = 0x00;
		self.wx = 0x00;
	}

	/// Update the ppu's state according to the elapsed time.
	pub fn process(&mut self, cycles: usize) {
		if !self.lcdc.power() {
			// LCD is powered off.
			return;
		}

		self.mode_counter += cycles;

		match self.mode {
			// Searching OAM
			PpuMode::SearchOam => {
				// Enter scanline if finished
				if self.mode_counter >= 80 {
					self.mode_counter -= 80;
					self.mode = PpuMode::RenderLine;
				}
			}

			PpuMode::RenderLine => {
				if self.mode_counter >= 172 {
					self.mode_counter -= 172;
					self.render_line();
					self.mode = PpuMode::Hblank;

					// Check if should prompt an interrupt when getting to Hblank mode.
					if self.stat.hblank_check_enable() {
						self.interrupt_flag |= Interrupt::LcdStat.value();
					}
				}
			}

			PpuMode::Hblank => {
				if self.mode_counter >= 204 {
					self.mode_counter -= 204;
					// Move to the next line
					self.ly += 1;
					// Set the concidence flag
					self.stat.set_lyc_signal(self.lyc == self.ly);

					if self.ly == 144 {
						// Start V-Blank.
						self.mode = PpuMode::Vblank;
						self.interrupt_flag |= Interrupt::VerticalBlank.value();
					} else {
						self.mode = PpuMode::SearchOam;
					}
				}
			}

			PpuMode::Vblank => {
				if self.mode_counter >= 456 {
					self.mode_counter -= 456;
					// Move to the next line
					self.ly += 1;
					self.stat.set_lyc_signal(self.lyc == self.ly);

					// TODO Make sure that it's actually 154 (it might be 153)
					if self.ly == 154 {
						// Start searching OAM
						self.ly = 0;
						self.stat.set_lyc_signal(self.lyc == self.ly);
						self.mode = PpuMode::SearchOam;

						// Check if should prompt an interrupt when getting to SearchOam mode.
						if self.stat.oam_check_enable() {
							self.interrupt_flag |= Interrupt::LcdStat.value();
						}
					}
				}
			}
		}
	}

	/// Perform the ppu's line rendering logic.
	pub fn render_line(&mut self) {
		// TODO implement this.
	}
}

impl Memory for Ppu {
	fn write(&mut self, address: u16, value: u8) -> Result<(), GameboyError> {
		match address {
			IO_LCDC => { self.lcdc.write(value); }
			IO_STAT => { self.stat.write(value); }
			IO_SCY => { self.scy = value; }
			IO_SCX => { self.scx = value; }
			IO_LYC => { self.lyc = value; }
			IO_BGP => { self.bgp = value; }
			IO_OBP0 => { self.obp0 = value; }
			IO_OBP1 => { self.obp1 = value; }
			IO_WY => { self.wy = value; }
			IO_WX => { self.wx = value; }
			memory_range!(MMAP_VIDEO_RAM) => {
				// Make sure that vram is currently writable
				assert!(self.mode != PpuMode::RenderLine);

				let offset = address as usize - range_start!(MMAP_VIDEO_RAM);
				self.vram[offset] = value;
			}
			_ => panic!("Ppu::write: register {:x} is not implemented", address)
		}

		Ok(())
	}

	fn read(&self, address: u16) -> Result<u8, GameboyError> {
		let result = match address {
			IO_LCDC => { self.lcdc.read() }
			IO_STAT => { self.stat.read() }
			IO_SCY => { self.scy }
			IO_SCX => { self.scx }
			IO_LY => { self.ly }
			IO_LYC => { self.lyc }
			IO_BGP => { self.bgp }
			IO_OBP0 => { self.obp0 }
			IO_OBP1 => { self.obp1 }
			IO_WY => { self.wy }
			IO_WX => { self.wx }
			memory_range!(MMAP_VIDEO_RAM) => {
				// Make sure that vram is currently readable
				assert!(self.mode != PpuMode::RenderLine);

				let offset = address as usize - range_start!(MMAP_VIDEO_RAM);
				self.vram[offset]
			}
			_ => panic!("Ppu::read: register {:x} is not implemented", address)
		};

		Ok(result)
	}
}

impl InterruptSource for Ppu {
	fn interrupts(&self) -> InterruptMask {
		self.interrupt_flag
	}

	fn clear(&mut self) {
		self.interrupt_flag = 0;
	}
}

#[allow(unused)]
impl Lcdc {
	pub fn new() -> Self {
		Lcdc { data: 0 }
	}

	pub fn reset(&mut self) {
		self.data = 0x91;
	}

	pub fn power(&self) -> bool {
		self.data & 0x80 != 0
	}

	pub fn window_tilemap(&self) -> bool {
		self.data & 0x40 != 0
	}

	pub fn window_enable(&self) -> bool {
		self.data & 0x20 != 0
	}

	/// 0 - 0x8800-0x97FF, 1 - 0x8000-0x8FFF
	pub fn tileset(&self) -> bool {
		self.data & 0x10 != 0
	}

	/// 0 - 0x9800-0x9BFF, 1 - 0x9C00-0x9FFF
	pub fn bg_tilemap(&self) -> bool {
		self.data & 0x8 != 0
	}

	/// 0 - 8x8, 1 - 8x16
	pub fn sprite_size(&self) -> bool {
		self.data & 0x4 != 0
	}

	pub fn sprites_enable(&self) -> bool {
		self.data & 0x2 != 0
	}

	pub fn bg_enable(&self) -> bool {
		self.data & 0x1 != 0
	}

	pub fn write(&mut self, value: u8) {
		self.data = value;
	}

	pub fn read(&self) -> u8 {
		self.data
	}
}

#[allow(unused)]
impl Stat {
	pub fn new() -> Self {
		Stat { data: 0, signal: 0, mode: 0 }
	}

	pub fn reset(&mut self) {
		self.data = 0;
	}

	pub fn lyc_check_enable(&self) -> bool {
		self.data & 0x40 != 0
	}

	pub fn oam_check_enable(&self) -> bool {
		self.data & 0x20 != 0
	}

	pub fn vblank_check_enable(&self) -> bool {
		self.data & 0x10 != 0
	}

	pub fn hblank_check_enable(&self) -> bool {
		self.data & 0x8 != 0
	}

	pub fn set_lyc_signal(&mut self, value: bool) {
		self.signal = value.into();
	}

	pub fn set_mode(&mut self, mode: PpuMode) {
		match mode {
			PpuMode::Hblank => { self.mode = 0; }
			PpuMode::Vblank => { self.mode = 1; }
			PpuMode::SearchOam => { self.mode = 2; }
			PpuMode::RenderLine => { self.mode = 3; }
		}
	}

	pub fn write(&mut self, value: u8) {
		self.data = value & !7;
	}

	pub fn read(&self) -> u8 {
		self.data | self.signal | self.mode
	}
}
