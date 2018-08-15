use time;

use hardware::Hardware;
use cpu::{CPU, R};

// time related interrupts
pub fn handle(cpu: &mut CPU, _hw: &mut Hardware) {
    match cpu.get_r8(R::AH) {
        0x00 => {
            // TIME - GET SYSTEM TIME
            // Return:
            // CX:DX = number of clock ticks since midnight
            // AL = midnight flag, nonzero if midnight passed since time last read
            if cpu.deterministic {
                cpu.set_r16(R::CX, 0);
                cpu.set_r16(R::DX, 0);
                cpu.set_r8(R::AL, 0);
            } else {
                println!("XXX FIXME - INT 1A GET TIME: return number of clock ticks since midnight");
                cpu.set_r16(R::CX, 1);
                cpu.set_r16(R::DX, 1);
                cpu.set_r8(R::AL, 0);
            }
        }
        0x01 => {
            // TIME - SET SYSTEM TIME
            // CX:DX = number of clock ticks since midnight
            let cx = cpu.get_r16(R::CX);
            let dx = cpu.get_r16(R::DX);
            println!("XXX SET SYSTEM TIME: CX:DX = {:04X}:{:04X}", cx, dx);
        }
        _ => {
            println!("int1a error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(R::AH),
                     cpu.get_r16(R::AX));
        }
    }
}
