use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Instant;

use crate::cpu::{CPU, R};
use crate::machine::Machine;

// time related interrupts
pub fn handle(machine: &mut Machine) {
    match machine.cpu.get_r8(R::AH) {
        0x00 => {
            // TIME - GET SYSTEM TIME
            // Return:
            // CX:DX = number of clock ticks since midnight
            // AL = midnight flag, nonzero if midnight passed since time last read
            if machine.cpu.deterministic {
                machine.cpu.set_r16(R::CX, 0);
                machine.cpu.set_r16(R::DX, 0);
                machine.cpu.set_r8(R::AL, 0);
            } else {
                // println!("INT 1A GET TIME: get number of clock ticks since midnight, ticks {}",  hw.pit.timer0.count);
                let cx = (machine.pit.timer0.count >> 16) as u16;
                let dx = (machine.pit.timer0.count & 0xFFFF) as u16;
                machine.cpu.set_r16(R::CX, cx);
                machine.cpu.set_r16(R::DX, dx);
                machine.cpu.set_r8(R::AL, 0); // TODO implement midnight flag
            }
        }
        0x01 => {
            // TIME - SET SYSTEM TIME
            // CX:DX = number of clock ticks since midnight
            let cx = machine.cpu.get_r16(R::CX);
            let dx = machine.cpu.get_r16(R::DX);
            let ticks = (u32::from(cx)) << 16 | u32::from(dx);

            machine.pit.timer0.count = ticks;
            // println!("SET SYSTEM TIME to {}", ticks);
        }
        _ => {
            println!("int1a (time) error: unknown ah={:02X}, ax={:04X}",
                     machine.cpu.get_r8(R::AH),
                     machine.cpu.get_r16(R::AX));
        }
    }
}
