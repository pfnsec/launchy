use super::Button;


#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Message {
	Press { button: Button },
	Release { button: Button },
	TextEndedOrLooped,
	/// Every once in a while, the device randomly spews out a weird undocumented MIDI message.
	/// I have no idea what that is about. It comes relatively regularly though, so I don't want
	/// to discard those messages either.
	UnknownShortMessage { bytes: [u8; 3] },
}

fn decode_grid_button(btn: u8) -> Button {
	return Button::GridButton { x: btn % 16, y: btn / 16 };
}

pub struct Input;

impl crate::InputDevice for Input {
	const MIDI_CONNECTION_NAME: &'static str = "Launchy S input";
	const MIDI_DEVICE_KEYWORD: &'static str = "Launchpad S";
	type Message = Message;

	fn decode_message(_timestamp: u64, data: &[u8]) -> Message {
		// first byte of a launchpad midi message is the message type
		return match data {
			&[0x90, button, velocity] => { // Note on
				let button = decode_grid_button(button);
				
				match velocity {
					0 => Message::Release { button },
					127 => Message::Press { button },
					other => panic!("Unexpected grid note-on velocity {}", other),
				}
			},
			&[0xB0, number @ 104..=111, velocity] => { // Controller change
				let button = Button::ControlButton { index: number - 104 };

				match velocity {
					0 => Message::Release { button },
					127 => Message::Press { button },
					other => panic!("Unexpected control note-on velocity {}", other),
				}
			},
			&[0xB0, 0, 3] => Message::TextEndedOrLooped,
			&[a, b, c] => Message::UnknownShortMessage { bytes: [a, b, c] },
			// YES we have no note off message handler here because it's not used by the launchpad.
			// It sends zero-velocity note-on messages instead.
			other => panic!("Unexpected midi message: {:?}", other),
		};
	}
}
