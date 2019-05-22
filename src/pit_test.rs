use crate::machine::Machine;

/*
#[test]
fn can_execute_pit_set_reload_value() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB0, 0x34,         // mov al,0b0011_0100   ; channel 0, lobyte/hibyte, rate generator
        0xE6, 0x43,         // out 0x43,al
        0xB8, 0x44, 0x22,   // mov ax,0x2244        ; ax = 16 bit reload value
        0xE6, 0x40,         // out 0x40,al          ; set low byte of PIT reload value
        0x88, 0xE0,         // mov al,ah            ; ax = high 8 bits of reload value
        0xE6, 0x40,         // out 0x40,al          ; set high byte of PIT reload value
    ];
    machine.load_executable(&code);
    machine.execute_instructions(6);

    // XXX need a new mechanism to read component registers
    assert_eq!(0x2244, machine.pit.timer0.reload);
}
*/
