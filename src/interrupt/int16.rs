use crate::hardware::Hardware;
use crate::cpu::{CPU, R};
use crate::cpu::*;

const DEBUG_KEYBOARD: bool = false;

// keyboard related interrupts
pub fn handle(cpu: &mut CPU, hw: &mut Hardware) {
    match cpu.get_r8(R::AH) {
        0x00 => {
            // KEYBOARD - GET KEYSTROKE
            let (ah, al) = hw.keyboard.consume_dos_standard_scancode_and_ascii();

            // AH = BIOS scan code
            // AL = ASCII character
            cpu.set_r8(R::AH, ah);
            cpu.set_r8(R::AL, al);

            if DEBUG_KEYBOARD {
                println!("KEYBOARD - GET KEYSTROKE, returns ah {:02x}, al {:02x}", ah, al);
            }
        }
        0x01 => {
            // KEYBOARD - CHECK FOR KEYSTROKE
            let (ah, al, _) = hw.keyboard.peek_dos_standard_scancode_and_ascii();

            // AH = BIOS scan code
            // AL = ASCII character
            cpu.set_r8(R::AH, ah);
            cpu.set_r8(R::AL, al);

            // ZF set if no keystroke available
            hw.bios.set_flag(&mut hw.mmu, FLAG_ZF, ah == 0);

            if DEBUG_KEYBOARD {
                println!("KEYBOARD - CHECK FOR KEYSTROKE, returns ah {:02x}, al {:02x}", ah, al);
            }
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
        0x92 => {
            // KEYB.COM KEYBOARD CAPABILITIES CHECK (not an actual function!)

            // Return:
            // AH <= 80h if enhanced keyboard functions (AH=10h-12h) supported
            cpu.set_r8(R::AH, 0x80); // indicates support
        }
        _ => {
            println!("int16 (keyboard) error: unknown ah={:02X}, ax={:04X}",
                     cpu.get_r8(R::AH),
                     cpu.get_r16(R::AX));
        }
    }
}
