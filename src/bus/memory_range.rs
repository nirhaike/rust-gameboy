// Copyright 2021 Nir H. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![deny(missing_docs)]
//! Macros for describing memory ranges.

/// Memory range is represented by start and end addresses, 16 bits each.
pub type MemoryRange = u32;

/// Memory boundries - wrapper for passing constants into a range pattern.
pub trait MemoryBounds<const S: u32> {
	/// The start address of the memory boundry.
	const START: u16 = ((S >> 16) & 0xFFFF) as u16;
	/// The end address of the memory boundry.
	const END: u16 = (S & 0xFFFF) as u16;
}

impl<const S: u32> MemoryBounds<S> for () { }

/// Make a memory range constant.
///
/// * `start` - Start address (inclusive).
/// * `end` - End address (inclusive).
#[macro_export]
macro_rules! make_range {
	($start:tt, $end:tt) => (($start << 16) + $end)
}

/// Returns the first address in the given memory range.
#[macro_export]
macro_rules! range_start {
	($range:tt) => { (<() as MemoryBounds<$range>>::START as usize) }
}

/// Returns the last address (inclusive) in the given memory range.
#[macro_export]
macro_rules! range_end {
	($range:tt) => { (<() as MemoryBounds<$range>>::END as usize) }
}

/// Create a range pattern from the given memory range.
///
/// # Examples
/// ```
/// # #[macro_use] extern crate gameboy_core;
/// # use gameboy_core::bus::memory_range::*;
/// # fn main() {
///
/// const MMAP_ROM_BANK0: MemoryRange = make_range!(0, 0x3FFF);
/// let address: u16 = 0x2000;
///
/// match address {
///		memory_range!(MMAP_ROM_BANK0) => {}
///		_ => { assert!(false); }
/// }
///
/// # }
/// ```
#[macro_export]
macro_rules! memory_range {
	($range:tt) => {
		<() as MemoryBounds<$range>>::START..=<() as MemoryBounds<$range>>::END
	}
}

/// Creates a range pattern of type `usize`.
/// Works somewhat like the `memory_range` macro, but suitable for array indexing.
#[macro_export]
macro_rules! memory_offset_range {
	($range:tt) => { range_start!($range)..=range_end!($range) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_range() {
    	const FIRST_5_BYTES: MemoryRange = make_range!(0, 4);

		match 3 {
			memory_range!(FIRST_5_BYTES) => {}
			_ => { assert!(false); }
		};

		match 5 {
			memory_range!(FIRST_5_BYTES) => { assert!(false); }
			_ => {}
		};
    }
}
