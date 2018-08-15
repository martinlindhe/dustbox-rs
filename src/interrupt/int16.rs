use hardware::Hardware;
use cpu::{CPU, R};
use cpu::*;

// keyboard related interrupts
pub fn handle(cpu: &mut CPU, hw: &mut Hardware) {
    match cpu.get_r8(R::AH) {
        0x00 => {
            // KEYBOARD - GET KEYSTROKE
            // Return:
            // AH = BIOS scan code
            // AL = ASCII character
            cpu.set_r16(R::AX, 0); // XXX
            println!("XXX impl KEYBOARD - GET KEYSTROKE");
        }
        0x01 => {
            // KEYBOARD - CHECK FOR KEYSTROKE
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character

            // println!("XXX impl KEYBOARD - CHECK FOR KEYSTROKE");
            hw.bios.set_flag(&mut hw.mmu, FLAG_ZF, true);
        }
        0x11 => {
            // KEYBOARD - CHECK FOR ENHANCED KEYSTROKE (enh kbd support only)
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character
            println!("XXX impl KEYBOARD - CHECK FOR ENHANCED KEYSTROKE");
            hw.bios.set_flag(&mut hw.mmu, FLAG_ZF, true);
        }
        _ => {
            println!("int16 error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(R::AH),
                     cpu.get_r16(R::AX));
        }
    }
}
