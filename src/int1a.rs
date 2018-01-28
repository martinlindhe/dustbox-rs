use time;

use cpu::CPU;
use register::{AX, BX, CX, DX, AL, ES};

// time related interrupts
pub fn handle(cpu: &mut CPU) {
    match cpu.r16[AX].hi_u8() {
        0x00 => {
            // TIME - GET SYSTEM TIME
            // Return:
            // CX:DX = number of clock ticks since midnight
            // AL = midnight flag, nonzero if midnight passed since time last read
            if cpu.deterministic {
                cpu.r16[CX].val = 0;
                cpu.r16[DX].val = 0;
                cpu.r16[AX].set_lo(0);
            } else {
                println!("XXX FIXME - INT 1A GET TIME: return number of clock ticks since midnight");
                cpu.r16[CX].val = 1;
                cpu.r16[DX].val = 1;
            }
        }
        _ => {
            println!("int1a error: unknown AH={:02X}, AX={:04X}",
                     cpu.r16[AX].hi_u8(),
                     cpu.r16[AX].val);
        }
    }
}
