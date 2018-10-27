use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Instant;

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
                let mut now = chrono::Local::now();
                let midnight = now.date().and_hms(0, 0, 0);

                // seconds since midnight
                let duration = now.signed_duration_since(midnight).to_std().unwrap();
                let seconds = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;

                // there are approximately 18.2 clock ticks per second, 0x18_00B0 per 24 hrs
                let ticks = (18.2 * seconds as f64) as u32;
                let cx = (ticks >> 16) as u16;
                let dx = (ticks & 0xFFFF) as u16;

                // println!("INT 1A GET TIME: return number of clock ticks since midnight   ticks {:?} = {:04X}:{:04X}",  ticks, cx, dx);
                cpu.set_r16(R::CX, cx);
                cpu.set_r16(R::DX, dx);
                cpu.set_r8(R::AL, 0); // TODO implement
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
            println!("int1a (time) error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(R::AH),
                     cpu.get_r16(R::AX));
        }
    }
}
