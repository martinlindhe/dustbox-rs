use sdl2::keyboard::{Keycode, Mod};

use crate::keyboard::{Keyboard, StatusRegister};
use crate::machine::Component;

#[test]
fn test_status_register() {
    let sr = StatusRegister::default();
    assert_eq!(0b001_0100, sr.as_u8()); // system 1, unknown4 1
}

#[test]
fn can_read_keys_from_io_ports() {
    let mut keyboard = Keyboard::default();

    // in al,0x64
    assert_eq!(Some(0x14), keyboard.in_u8(0x64));

    // inject key press
    keyboard.add_keypress(Keycode::Escape, Mod::NOMOD);

    // in al,0x64
    assert_eq!(Some(0x15), keyboard.in_u8(0x64));

    // make sure we get the DOS scancode for ESC key

    // in al,0x60
    assert_eq!(Some(0x01), keyboard.in_u8(0x60));
}

#[test]
fn consumes_keypress_queue() {
    let mut keyboard = Keyboard::default();

    assert_eq!(false, keyboard.has_queued_presses());

    // inject key press
    keyboard.add_keypress(Keycode::Escape, Mod::NOMOD);
    keyboard.add_keypress(Keycode::Escape, Mod::NOMOD);
    assert_eq!(true, keyboard.has_queued_presses());

    // read it
    let (_, _, keypress) = keyboard.peek_dos_standard_scancode_and_ascii();
    let keypress = keypress.unwrap();

    // consume 1st
    keyboard.consume(&keypress);
    assert_eq!(true, keyboard.has_queued_presses());

    // consume 2nd
    keyboard.consume(&keypress);
    assert_eq!(false, keyboard.has_queued_presses());
}
