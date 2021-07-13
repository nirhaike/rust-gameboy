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
	pub const OAM_SIZE: usize = 0xa0;

	pub const NUM_SPRITES: usize = 40;

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
#[derive(Clone, Copy, Debug, PartialEq)]
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

struct SpriteData {
	x: u8,
	y: u8,
	tile_id: u8,
	tile_attr: u8,
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

	/// Writes the display's output to the given frame buffer.
	pub fn flush(&mut self, frame_buffer: &mut [u32]) {
		frame_buffer.copy_from_slice(&self.buffer);
	}

	/// Getter for the OAM region's buffer.
	pub fn oam(&mut self) -> &mut [u8] {
		&mut self.oam
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
					self.set_mode(PpuMode::RenderLine);
				}
			}

			PpuMode::RenderLine => {
				if self.mode_counter >= 172 {
					self.mode_counter -= 172;
					self.render_line();
					self.set_mode(PpuMode::Hblank);

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
					self.refresh_lyc_signal();

					if self.ly == 144 {
						// Start V-Blank.
						self.set_mode(PpuMode::Vblank);
						self.interrupt_flag |= Interrupt::VerticalBlank.value();
						// Check if should prompt an interrupt when getting to V-blank mode.
						if self.stat.vblank_check_enable() {
							self.interrupt_flag |= Interrupt::LcdStat.value();
						}
					} else {
						self.set_mode(PpuMode::SearchOam);
					}
				}
			}

			PpuMode::Vblank => {
				if self.mode_counter >= 456 {
					self.mode_counter -= 456;
					// Move to the next line
					self.ly += 1;
					self.refresh_lyc_signal();

					// TODO Make sure that it's actually 154 (it might be 153)
					if self.ly == 154 {
						// Start searching OAM
						self.ly = 0;
						self.refresh_lyc_signal();
						self.set_mode(PpuMode::SearchOam);

						// Check if should prompt an interrupt when getting to SearchOam mode.
						if self.stat.oam_check_enable() {
							self.interrupt_flag |= Interrupt::LcdStat.value();
						}
					}
				}
			}
		}
	}

	fn set_mode(&mut self, mode: PpuMode) {
		self.mode = mode;
		self.stat.set_mode(mode);
	}

	fn refresh_lyc_signal(&mut self) {
		self.stat.set_lyc_signal(self.lyc == self.ly);

		if self.stat.signal != 0 && self.stat.lyc_check_enable() {
			self.interrupt_flag |= Interrupt::LcdStat.value();
		}
	}

	/// Perform the ppu's line rendering logic.
	fn render_line(&mut self) {
		let line_offset = (self.ly as usize) * WIDTH;

		// Wipe the buffer's line
		for x in 0..WIDTH {
			self.buffer[line_offset + x] = PALETTE[0];
		}

		self.draw_bg();
		self.draw_sprites();
	}

	fn draw_bg(&mut self) {
		if !self.lcdc.bg_enable() && !self.lcdc.window_enable() {
			return;
		}

		// Calculate the offset of the current height in the frame buffer.
		let line_offset = (self.ly as usize) * WIDTH;

		// Select between displaying window or background.
		let show_window = self.lcdc.window_enable() && self.wy < self.ly;

		let wx = self.wx.wrapping_sub(7);
		let screen_y = if show_window { self.ly.wrapping_sub(self.wy) } else { self.scy.wrapping_add(self.ly) };
		let tile_y = ((screen_y as u16) >> 3) & 31;

		// Iterate over the current line in the x-axis and draw the pixels.
		for x in 0..WIDTH {
			let screen_x = if show_window && x as u8 >= wx { x as u8 - wx } else { self.scx.wrapping_add(x as u8) };
			let tile_x = ((screen_x as u16) >> 3) & 31;

			// Get the base offset of the background.
			let base_offset = [0x1800, 0x1c00][
				if show_window && x as u8 >= wx {
					if self.lcdc.window_tilemap() { 1 } else { 0 }
				} else if self.lcdc.bg_tilemap() {
					1
				} else {
					0
				}];

			// The tile takes 2 bytes for each line.
			let tile_number_offset = (base_offset + tile_y * 32 + tile_x) as usize;
			let tile_number = self.vram[tile_number_offset];
			let tile_offset = if self.lcdc.tileset() {
				tile_number as usize
			} else {
				((tile_number as i8) as usize).wrapping_add(128)
			} as usize * 16;

			let tileset_select = if self.lcdc.tileset() { 0 } else { 0x800 };
			let tile_data_offset = (tileset_select + tile_offset) as usize + (screen_y as usize % 8) * 2;
			let tile_data = &self.vram[tile_data_offset..tile_data_offset+2];

			let tile_x = screen_x % 8;

			// Get the color from the background's palette.
			let color_low = if tile_data[0] & (0x80 >> tile_x) != 0 { 1 } else { 0 };
			let color_high = if tile_data[1] & (0x80 >> tile_x) != 0 { 2 } else { 0 };
			let color_index = color_high | color_low;

			let color = Ppu::get_color(self.bgp, color_index);
			self.buffer[line_offset + x] = PALETTE[color];
		}
	}

	fn draw_sprites(&mut self) {
		let line_offset = (self.ly as usize) * WIDTH;
		// Determine the sprite height (width is always 8)
		let sprite_height = if self.lcdc.sprite_size() { 16 } else { 8 };

		for i in 0..NUM_SPRITES {
			let sprite_addr = (i as usize) * 4;
			let sprite_data = SpriteData::new(&self.oam[sprite_addr..sprite_addr+4],
											  self.lcdc.sprite_size());

			// Check whether the sprite is out of bounds
			let oob_x = sprite_data.x >= (WIDTH as u8) && sprite_data.x <= (0xff - 7);
			let oob_ly_down = self.ly < sprite_data.y || self.ly > sprite_data.y.wrapping_add(sprite_height).wrapping_sub(1);
			let oob_ly_up = self.ly > sprite_data.y.wrapping_add(sprite_height).wrapping_sub(1);
			let sprite_wrapping_y = sprite_data.y > 0xff - sprite_height + 1;

			// Continue if the sprite is not relevant for the current line.
			if oob_x ||
			   (sprite_wrapping_y && oob_ly_up) ||
			   (!sprite_wrapping_y && oob_ly_down) {
				continue;
			}

			let tile_y = if sprite_data.flip_y() {
				sprite_height - 1 - self.ly.wrapping_sub(sprite_data.y)
			} else {
				self.ly.wrapping_sub(sprite_data.y)
			};

			// The tile takes 2 bytes for each line.
			let tile_data_offset = (sprite_data.tile_id as usize) * 16 + (tile_y as usize) * 2;
			let tile_data = &self.vram[tile_data_offset..tile_data_offset+2];

			// Draw the relevant pixels in the current line.
			for x in 0..8 {
				let pixel_x = sprite_data.x.wrapping_add(x);
				let tile_x = if sprite_data.flip_x() { 7 - x } else { x };

				let color_low = if tile_data[0] & (0x80 >> tile_x) != 0 { 1 } else { 0 };
				let color_high = if tile_data[1] & (0x80 >> tile_x) != 0 { 2 } else { 0 };
				let color_index = color_high | color_low;

				// Don't draw invisible and off-screen pixels.
				if color_index == 0 || pixel_x >= (WIDTH as u8) {
					continue;
				}

				let active_palette = if sprite_data.palette_select() {
					self.obp1
				} else {
					self.obp0
				};
				let color = Ppu::get_color(active_palette, color_index);

				// Draw the pixel
				let offset = line_offset + sprite_data.x.wrapping_add(x) as usize;

				if !sprite_data.sprite_behind() || self.buffer[offset] != PALETTE[0] {
					self.buffer[offset] = PALETTE[color];
				}
			}
		}
	}

	fn get_color(palette: u8, color: u8) -> usize {
		match palette >> (2 * color) & 0x03 {
			0x00 => 3,
			0x01 => 2,
			0x02 => 1,
			_ => 0,
		}
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
				// TODO fix ppu timing and enable this assertion.
				// assert!(self.mode != PpuMode::RenderLine);

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
				// TODO fix ppu timing and enable this assertion.
				// assert!(self.mode != PpuMode::RenderLine);

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
		self.signal = (value as u8) << 2;
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
		// Bit 7 is unused and always returns 1.
		self.data | self.signal | self.mode | 0x80
	}
}

impl SpriteData {
	pub fn new(data: &[u8], sprite_size: bool) -> Self {
		assert!(data.len() == 4);

		SpriteData {
			x: data[1].wrapping_sub(8),
			y: data[0].wrapping_sub(16),
			tile_id: data[2] & if sprite_size { 0xfe } else { 0xff },
			tile_attr: data[3],
		}
	}

	pub fn palette_select(&self) -> bool {
		self.tile_attr & (1 << 4) != 0
	}

	pub fn flip_x(&self) -> bool {
		self.tile_attr & (1 << 5) != 0
	}

	pub fn flip_y(&self) -> bool {
		self.tile_attr & (1 << 6) != 0
	}

	pub fn sprite_behind(&self) -> bool {
		self.tile_attr & (1 << 7) != 0
	}
}
