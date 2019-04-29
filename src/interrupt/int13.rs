use crate::hardware::Hardware;
use crate::cpu::{CPU, R};
use crate::cpu::*;

// disk related interrupts
pub fn handle(cpu: &mut CPU, _hw: &mut Hardware) {
    match cpu.get_r8(R::AH) {
        0x00 => {
            // DISK - RESET DISK SYSTEM
            // DL = drive (if bit 7 is set both hard disks and floppy disks reset)
            println!("XXX DISK - RESET DISK SYSTEM, dl={:02X}", cpu.get_r8(R::DL))
            // Return:
            // AH = status (see #00234)
            // CF clear if successful (returned AH=00h)
            // CF set on error
        }
        _ => {
            println!("int13 (disk) error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(R::AH),
                     cpu.get_r16(R::AX));
        }
    }
}
