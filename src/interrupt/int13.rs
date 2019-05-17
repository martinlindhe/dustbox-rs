use crate::cpu::{CPU, R};
use crate::cpu::*;
use crate::machine::Machine;

// disk related interrupts
pub fn handle(machine: &mut Machine) {
    match machine.cpu.get_r8(R::AH) {
        0x00 => {
            // DISK - RESET DISK SYSTEM
            // DL = drive (if bit 7 is set both hard disks and floppy disks reset)
            println!("XXX DISK - RESET DISK SYSTEM, dl={:02X}", machine.cpu.get_r8(R::DL))
            // Return:
            // AH = status (see #00234)
            // CF clear if successful (returned AH=00h)
            // CF set on error
        }
        _ => {
            println!("int13 (disk) error: unknown ah={:02X}, ax={:04X}",
                     machine.cpu.get_r8(R::AH),
                     machine.cpu.get_r16(R::AX));
        }
    }
}
