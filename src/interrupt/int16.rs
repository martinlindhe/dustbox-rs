use hardware::Hardware;
use cpu::CPU;
use cpu::register::{R8, R16};

// keyboard related interrupts
pub fn handle(cpu: &mut CPU, _hw: &mut Hardware) {
    match cpu.get_r8(&R8::AH) {
        0x00 => {
            // KEYBOARD - GET KEYSTROKE
            // Return:
            // AH = BIOS scan code
            // AL = ASCII character
            cpu.set_r16(&R16::AX, 0); // XXX
            println!("XXX impl KEYBOARD - GET KEYSTROKE");
        }
        0x01 => {
            // KEYBOARD - CHECK FOR KEYSTROKE
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character

            //println!("XXX impl KEYBOARD - CHECK FOR KEYSTROKE");
            cpu.flags.zero = true;
        }
        0x11 => {
            // KEYBOARD - CHECK FOR ENHANCED KEYSTROKE (enh kbd support only)
            // Return:
            // ZF set if no keystroke available
            // ZF clear if keystroke available
            // AH = BIOS scan code
            // AL = ASCII character
            println!("XXX impl KEYBOARD - CHECK FOR ENHANCED KEYSTROKE");
            cpu.flags.zero = true;
        }
        _ => {
            println!("int16 error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(&R8::AH),
                     cpu.get_r16(&R16::AX));
        }
    }
}
