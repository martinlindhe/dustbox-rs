use crate::cpu::{CPU, R};
use crate::machine::Machine;

// mouse related interrupts
pub fn handle(machine: &mut Machine) {
    match machine.cpu.get_r16(R::AX) {
        0x0003 => {
            // MS MOUSE v1.0+ - RETURN POSITION AND BUTTON STATUS
            // Return:
            // BX = button status (see #03168)
            // CX = column
            // DX = row
            // Note: In text modes, all coordinates are specified as multiples of the cell size, typically 8x8 pixels 
            println!("XXX impl MOUSE - RETURN POSITION AND BUTTON STATUS");
        }
        _ => {
            println!("int33 (mouse) error: unknown ax={:04X}, ip={:04X}:{:04X}",
                     machine.cpu.get_r16(R::AX),
                     machine.cpu.get_r16(R::CS),
                     machine.cpu.regs.ip);
        }
    }
}
