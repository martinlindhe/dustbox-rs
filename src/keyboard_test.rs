use sdl2::keyboard::{Keycode, Mod};

use crate::keyboard::{Keyboard, StatusRegister};
use crate::machine::Machine;
use crate::cpu::R;

#[test]
fn test_status_register() {
    let sr = StatusRegister::default();
    assert_eq!(0b001_0100, sr.as_u8()); // system 1, unknown4 1
}

/*
#[test]
fn can_read_keys_from_io_ports() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xE4, 0x64, // 00000100: in al,0x64
        0x24, 0x01, // 00000102: and al,0x1
        0x74, 0xFA, // 00000104: jz 0x100
        0xE4, 0x60, // 00000106: in al,0x60
    ];
    machine.load_executable(&code);
    machine.execute_instruction(); // in al,0x64
    assert_eq!(0x14, machine.cpu.get_r8(R::AL));

    // make sure still in loop
    machine.execute_instructions(2);
    assert_eq!(0x0100, machine.cpu.regs.ip);

    // XXX need a better way to address keyboard component

    // inject key press
    machine.keyboard.add_keypress(Keycode::Escape, Mod::NOMOD);

    // make sure we break the loop
    machine.execute_instruction(); // in al,0x64
    assert_eq!(0x15, machine.cpu.get_r8(R::AL));
    machine.execute_instructions(2);

    // make sure we get the DOS scancode for ESC key
    machine.execute_instruction(); // in al,0x60
    assert_eq!(0x01, machine.cpu.get_r8(R::AL));
}
*/

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
