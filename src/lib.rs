// WOW Clippy HATES being explicit
// and also, the reasoning against tabs in doc comments is exactly the same reasoning against tabs
// as indentation in general - and that is totally stupid, because indentation style is something
// subjective. Guess clippy missed the note about that.
#![allow(clippy::needless_return, clippy::tabs_in_doc_comments)]

mod util;

mod protocols;

mod color;
pub use color::*;

mod canvas;
pub use canvas::*;

mod midi_io;
pub use midi_io::*;

pub mod launchpad_s;
pub use launchpad_s as s;

pub mod launchpad_mk2;
pub use launchpad_mk2 as mk2;

pub mod launch_control;
pub use launch_control as control;

pub mod prelude {
	pub use crate::midi_io::{OutputDevice, InputDevice, MsgPollingWrapper};
	pub use crate::canvas::Canvas;
}

/// Identifier used for e.g. the midi port names etc.
const APPLICATION_NAME: &str = "LaunchpadRs";


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum Button {
	ControlButton { index: u8 },
	GridButton { x: u8, y: u8 },
}

impl Button {
	pub const UP: Self = Button::ControlButton { index: 0 };
	pub const DOWN: Self = Button::ControlButton { index: 1 };
	pub const LEFT: Self = Button::ControlButton { index: 2 };
	pub const RIGHT: Self = Button::ControlButton { index: 3 };
	pub const SESSION: Self = Button::ControlButton { index: 4 };
	pub const USER_1: Self = Button::ControlButton { index: 5 };
	pub const USER_2: Self = Button::ControlButton { index: 6 };
	pub const MIXER: Self = Button::ControlButton { index: 7 };
	pub const VOLUME: Self = Button::GridButton { x: 8, y: 0 };
	pub const PAN: Self = Button::GridButton { x: 8, y: 1 };
	pub const SEND_A: Self = Button::GridButton { x: 8, y: 2 };
	pub const SEND_B: Self = Button::GridButton { x: 8, y: 3 };
	pub const STOP: Self = Button::GridButton { x: 8, y: 4 };
	pub const MUTE: Self = Button::GridButton { x: 8, y: 5 };
	pub const SOLO: Self = Button::GridButton { x: 8, y: 6 };
	pub const RECORD_ARM: Self = Button::GridButton { x: 8, y: 7 };

	/// Creates a new button out of absolute coordinates, like the ones returned by `abs_x()` and
	/// `abs_y()`.
	pub fn from_abs(x: u8, y: u8) -> Button {
		match y {
			0 => {
				assert!(x <= 7);
				return Button::ControlButton { index: x };
			},
			1..=8 => {
				assert!(x <= 8);
				return Button::GridButton { x, y: y - 1 };
			},
			other => panic!("Unexpected y: {}", other),
		}
	}

	/// Returns x coordinate assuming coordinate origin in the leftmost control button
	pub fn abs_x(&self) -> u8 {
		match *self {
			Self::ControlButton { index } => return index,
			Self::GridButton { x, .. } => return x,
		}
	}

	/// Returns y coordinate assuming coordinate origin in the leftmost control button
	pub fn abs_y(&self) -> u8 {
		match *self {
			Self::ControlButton { .. } => return 0,
			Self::GridButton { y, .. } => y + 1,
		}
	}
}