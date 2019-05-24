use crate::machine::{Component, Machine};
use crate::pit::PIT;

#[test]
fn can_execute_pit_set_reload_value() {
    let mut pit = PIT::default();

    // mov al,0b0011_0100   ; channel 0, lobyte/hibyte, rate generator
    // out 0x43,al
    pit.out_u8(0x43, 0b0011_0100);

    // mov ax,0x2244
    // out 0x40,al          ; low byte of PIT reload value = 0x44
    pit.out_u8(0x40, 0x44);

    // mov al,ah
    // out 0x40,al          ; high byte of PIT reload value = 0x22
    pit.out_u8(0x40, 0x22);

    // XXX need a new mechanism to read component registers
    assert_eq!(0x2244, pit.timer0.reload);
}
