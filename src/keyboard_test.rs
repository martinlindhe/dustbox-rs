use sdl2::keyboard::{Keycode, Mod, NOMOD};

use keyboard::StatusRegister;
use machine::Machine;
use cpu::R;

#[test]
fn test_status_register() {
    let sr = StatusRegister::default();
    assert_eq!(0b001_0100, sr.as_u8()); // system 1, unknown4 1
}

#[test]
fn can_read_keys_from_io_ports() {
    let mut machine = Machine::default();
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

    // inject key press
    machine.hw.keyboard.add_keypress(Keycode::Escape, NOMOD);

    // make sure we break the loop
    machine.execute_instruction(); // in al,0x64
    assert_eq!(0x15, machine.cpu.get_r8(R::AL));
    machine.execute_instructions(2);

    // make sure we get the DOS scancode for ESC key
    machine.execute_instruction(); // in al,0x60
    assert_eq!(0x01, machine.cpu.get_r8(R::AL));
}
