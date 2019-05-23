// TODO later: dont depend on sdl2 in the core crate (process events with something else?)
use sdl2::keyboard::{Keycode, Mod};

use crate::cpu::{CPU, R};
use crate::machine::Component;

const DEBUG_KEYBOARD: bool = false;

#[cfg(test)]
#[path = "./keyboard_test.rs"]
mod keyboard_test;

#[derive(Clone)]
pub struct Keyboard {
    keypresses: Vec<Keypress>,
    status_register: StatusRegister,
}

/// https://wiki.osdev.org/%228042%22_PS/2_Controller#Status_Register
#[derive(Clone)]
struct StatusRegister {
    /// 0 = empty, 1 = full
    /// must be set before attempting to read data from IO port 0x60
    output_buffer_status: bool,

    /// 0 = empty, 1 = full
    /// must be clear before attempting to write data to IO port 0x60 or IO port 0x64
    input_buffer_status: bool,

    /// Meant to be cleared on reset and set by firmware (via. PS/2 Controller Configuration Byte) if the system passes self tests (POST)
    system: bool,

    /// Command/data
    /// 0 = data written to input buffer is data for PS/2 device
    /// 1 = data written to input buffer is data for PS/2 controller command)
    mode: bool,

    /// Unknown (chipset specific)
    /// is set in dosbox-x and WinXP
    unknown4: bool,

    /// Unknown (chipset specific)
    unknown5: bool,

    /// 0 = no error, 1 = time-out error
    timeout_error: bool,

    /// 0 = no error, 1 = parity error
    parity_error: bool,
}

impl StatusRegister {
    pub fn default() -> Self {
        StatusRegister {
            output_buffer_status: false,
            input_buffer_status: false,
            system: true,
            mode: false,
            unknown4: true,
            unknown5: false,
            timeout_error: false,
            parity_error: false,
        }
    }

    pub fn as_u8(&self) -> u8 {
        let mut res = 0;
        if self.output_buffer_status {
            res |= 1;
        }
        if self.input_buffer_status {
            res |= 2;
        }
        if self.system {
            res |= 4;
        }
        if self.mode {
            res |= 8;
        }
        if self.unknown4 {
            res |= 16;
        }
        if self.unknown5 {
            res |= 32;
        }
        if self.timeout_error {
            res |= 64;
        }
        if self.parity_error {
            res |= 128;
        }
        res
    }
}

impl Component for Keyboard {
    fn in_u8(&mut self, port: u16) -> Option<u8> {
        // PORT 0060-006F - KEYBOARD CONTROLLER 804x (8041, 8042) (or PPI (8255) on PC,XT)
        // Note: XT uses ports 60h-63h, AT uses ports 60h-64h
        match port {
            0x0060 => {
                // keyboard controller data output buffer
                let (scancode, _, keypress) = self.peek_dos_standard_scancode_and_ascii();
                if let Some(keypress) = keypress {
                    self.consume(&keypress);
                }
                Some(scancode)
            },
            0x0061 => {
                // keyboard controller port b control register
                let val = 0 as u8; // XXX
                println!("XXX impl -- keyboard: read keyboard controller port b control register (current {:02X})", val);
                Some(val)
            }
            0x0064 => {
                // keyboard controller read status
                Some(self.get_status_register_byte())
            }
            _ => None
        }
    }

    fn out_u8(&mut self, port: u16, data: u8) -> bool {
        match port {
            0x0061 => {
                // keyboard controller port b OR ppi programmable periphial interface (XT only) - which mode are we in?
                println!("XXX impl -- keyboard: write keyboard controller port b {:02X}", data);
            }
            _ => return false
        }
        true
    }

    fn int(&mut self, int: u8, cpu: &mut CPU) -> bool {
        if int != 0x16 {
            return false;
        }
        match cpu.get_r8(R::AH) {
            0x00 => {
                // KEYBOARD - GET KEYSTROKE
                let (ah, al) = self.consume_dos_standard_scancode_and_ascii();

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
                let (ah, al, _) = self.peek_dos_standard_scancode_and_ascii();

                // AH = BIOS scan code
                // AL = ASCII character
                cpu.set_r8(R::AH, ah);
                cpu.set_r8(R::AL, al);

                // ZF set if no keystroke available
                // machine.bios.set_flag(&mut machine.mmu, FLAG_ZF, ah == 0);
                cpu.regs.flags.zero = ah == 0;

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
                // machine.bios.set_flag(&mut machine.mmu, FLAG_ZF, true);
                cpu.regs.flags.zero = true;
            }
            0x92 => {
                // KEYB.COM KEYBOARD CAPABILITIES CHECK (not an actual function!)

                // Return:
                // AH <= 80h if enhanced keyboard functions (AH=10h-12h) supported
                cpu.set_r8(R::AH, 0x80); // indicates support
            }
            _ => return false
        }
        true
    }
}

/// Implements a PS/2 keyboard
/// https://wiki.osdev.org/PS/2_Keyboard
/// https://wiki.osdev.org/"8042"_PS/2_Controller
///
/// Usable test program for this is ../dos-software-decoding/demo-com-16bit/4sum/4sum.com
impl Keyboard {
    pub fn default() -> Self {
        Self {
            keypresses: Vec::new(),
            status_register: StatusRegister::default(),
        }
    }

    pub fn has_queued_presses(&self) -> bool {
        !self.keypresses.is_empty()
    }

    pub fn add_keypress(&mut self, keycode: Keycode, modifier: Mod) {
        let keypress = Keypress{keycode, modifier};
        if DEBUG_KEYBOARD {
            println!("keyboard: add_keypress {:?}", keypress);
        }
        self.keypresses.push(keypress);

        // signal there is bytes to be read
        self.status_register.output_buffer_status = true;
    }

    fn consume_keypress(&mut self) -> Keypress {
        self.keypresses.pop().unwrap()
    }

    fn peek_keypress(&self) -> Option<Keypress> {
        let len = self.keypresses.len();
        if len > 0 {
            let val = self.keypresses[len - 1].clone();
            Some(val)
        } else {
            None
        }
    }

    pub fn get_status_register_byte(&self) -> u8 {
        let val = self.status_register.as_u8();
        if DEBUG_KEYBOARD {
            println!("keyboard: read keyboard controller read status (current {:02X})", val);
        }
        val
    }

    /// used by int 0x16 function 0x00
    /// returns scancode, ascii, keypress
    pub fn consume_dos_standard_scancode_and_ascii(&mut self) -> (u8, u8) {
        let (ah, al, keypress) = self.peek_dos_standard_scancode_and_ascii();
        if let Some(keypress) = keypress {
            if DEBUG_KEYBOARD {
                println!("keyboard: consume_dos_standard_scancode_and_ascii consumes {:?}", keypress);
            }
            self.consume(&keypress);
        }
        (ah, al)
    }

    /// used by int 0x16 function 0x01
    /// returns scancode, ascii, keypress
    pub fn peek_dos_standard_scancode_and_ascii(&self) -> (u8, u8, Option<Keypress>) {
        if let Some(keypress) = self.peek_keypress() {
            let (ah, al) = map_sdl_to_dos_standard_codes(&keypress);
            if DEBUG_KEYBOARD {
                println!("keyboard: peek_dos_standard_scancode_and_ascii returns scancode {:02X}, ascii {:02X}, {:?}", ah, al, keypress);
            }
            (ah, al, Some(keypress))
        } else {
            if DEBUG_KEYBOARD {
                println!("keyboard: peek_dos_standard_scancode_and_ascii returns nothing");
            }
            (0, 0, None)
        }
    }

    fn find_keypress_index(&self, keypress: &Keypress) -> Option<usize> {
        for (idx, x) in self.keypresses.iter().enumerate() {
            println!("{}", x.keycode);
            if x == keypress {
                return Some(idx);
            }
        }
        None
    }

    pub fn consume(&mut self, keypress: &Keypress) {
        if DEBUG_KEYBOARD {
            println!("keyboard: consume {:?}", keypress);
        }

        if let Some(idx) = self.find_keypress_index(keypress) {
            self.keypresses.remove(idx);
        }

        println!("ERROR failed to consume keypress {:?}", keypress);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keypress {
    keycode: Keycode,
    modifier: Mod,
}

/// returns keycodes as specified in https://sites.google.com/site/pcdosretro/scancodes
impl Keypress {
    /// keycodes with no modifier key, returns scancode, ascii
    pub fn to_std_normal(&self) -> (u8, u8) {
        match self.keycode {
            Keycode::Escape => (0x01, 0x1B),
            Keycode::Num1 => (0x02, 0x31),
            Keycode::Num2 => (0x03, 0x32),
            Keycode::Num3 => (0x04, 0x33),
            Keycode::Num4 => (0x05, 0x34),
            Keycode::Num5 => (0x06, 0x35),
            Keycode::Num6 => (0x07, 0x36),
            Keycode::Num7 => (0x08, 0x37),
            Keycode::Num8 => (0x09, 0x38),
            Keycode::Num9 => (0x0A, 0x39),
            Keycode::Num0 => (0x0B, 0x30),
            Keycode::Minus => (0x0C, 0x2D),
            Keycode::Equals => (0x0D, 0x3D),
            Keycode::Backspace => (0x0E, 0x08),
            Keycode::Tab => (0x0F, 0x09),
            Keycode::Q => (0x10, 0x71),
            Keycode::W => (0x11, 0x77),
            Keycode::E => (0x12, 0x65),
            Keycode::R => (0x13, 0x72),
            Keycode::T => (0x14, 0x74),
            Keycode::Y => (0x15, 0x79),
            Keycode::U => (0x16, 0x75),
            Keycode::I => (0x17, 0x69),
            Keycode::O => (0x18, 0x6F),
            Keycode::P => (0x19, 0x70),
            Keycode::LeftBracket => (0x1A, 0x5B),  // XXX [
            Keycode::RightBracket => (0x1B, 0x5D), // XXX ]
            Keycode::Return => (0x1C, 0x0D),
            // 0x1D = CTRL but cant be read as its a modifier
            Keycode::A => (0x1E, 0x61),
            Keycode::S => (0x1F, 0x73),
            Keycode::D => (0x20, 0x64),
            Keycode::F => (0x21, 0x66),
            Keycode::G => (0x22, 0x67),
            Keycode::H => (0x23, 0x68),
            Keycode::J => (0x24, 0x6A),
            Keycode::K => (0x25, 0x6B),
            Keycode::L => (0x26, 0x6C),
            Keycode::Colon => (0x27, 0x3B), // XXX ; :
            Keycode::Quote => (0x28, 0x27), // XXX ' "
            Keycode::Caret => (0x29, 0x60), // XXX ` ~
            // 0x2A = Left Shift
            Keycode::Backslash => (0x2B, 0x5C), // XXX \ |
            Keycode::Z => (0x2C, 0x7A),
            Keycode::X => (0x2D, 0x78),
            Keycode::C => (0x2E, 0x63),
            Keycode::V => (0x2F, 0x76),
            Keycode::B => (0x30, 0x62),
            Keycode::N => (0x31, 0x6E),
            Keycode::M => (0x32, 0x6D),
            Keycode::Comma => (0x33, 0x2C), // XXX , <
            Keycode::Period => (0x34, 0x2E), // XXX . >
            Keycode::Slash => (0x35, 0x2F),  // XXX / ?
            // 0x36 = Right Shift
            Keycode::Asterisk => (0x37, 0x2A),
            // 0x38 = Alt
            Keycode::Space => (0x39, 0x20),
            // 0x3A = Caps Lock
            Keycode::F1 => (0x3B, 0x00),
            Keycode::F2 => (0x3C, 0x00),
            Keycode::F3 => (0x3D, 0x00),
            Keycode::F4 => (0x3E, 0x00),
            Keycode::F5 => (0x3F, 0x00),
            Keycode::F6 => (0x40, 0x00),
            Keycode::F7 => (0x41, 0x00),
            Keycode::F8 => (0x42, 0x00),
            Keycode::F9 => (0x43, 0x00),
            Keycode::F10 => (0x44, 0x00),
            // 0x45 = Num Lock
            // 0x46 = Scroll Lock
            Keycode::Home => (0x47, 0x00),
            Keycode::Up => (0x48, 0x00),
            Keycode::PageUp => (0x49, 0x00),
            Keycode::KpMinus => (0x4A, 0x2D), // XXX numeric keypad minus
            Keycode::Left => (0x4B, 0x00),
            // XXX Keycode::KpClearEntry => (0x00, 0x00),
            Keycode::Right => (0x4D, 0x00),
            Keycode::KpPlus => (0x4E, 0x2B), // XXX numeric keypad plus
            Keycode::End => (0x4F, 0x00),
            Keycode::Down => (0x50, 0x00),
            Keycode::PageDown => (0x51, 0x00),
            Keycode::Insert => (0x52, 0x00),
            Keycode::Delete => (0x53, 0x00),
            _ => {
                println!("unhandled NORMAL keycode mapping for {:#?}", self.keycode);
                (0, 0)
            }
        }
    }

    pub fn to_std_shift(&self) -> (u8, u8) {
        match self.keycode {
            Keycode::Escape => (0x01, 0x1B),
            Keycode::Num1 => (0x02, 0x21),
            Keycode::Num2 => (0x03, 0x40),
            Keycode::Num3 => (0x04, 0x23),
            Keycode::Num4 => (0x05, 0x24),
            Keycode::Num5 => (0x06, 0x25),
            Keycode::Num6 => (0x07, 0x5E),
            Keycode::Num7 => (0x08, 0x26),
            Keycode::Num8 => (0x09, 0x2A),
            Keycode::Num9 => (0x0A, 0x28),
            Keycode::Num0 => (0x0B, 0x29),
            Keycode::Minus => (0x0C, 0x5F),
            Keycode::Equals => (0x0D, 0x2B),
            Keycode::Backspace => (0x0E, 0x08),
            Keycode::Tab => (0x0F, 0x00),
            Keycode::Q => (0x10, 0x51),
            Keycode::W => (0x11, 0x57),
            Keycode::E => (0x12, 0x45),
            Keycode::R => (0x13, 0x52),
            Keycode::T => (0x14, 0x54),
            Keycode::Y => (0x15, 0x59),
            Keycode::U => (0x16, 0x55),
            Keycode::I => (0x17, 0x49),
            Keycode::O => (0x18, 0x4F),
            Keycode::P => (0x19, 0x50),
            Keycode::LeftBracket => (0x1A, 0x7B),  // XXX [ {
            Keycode::RightBracket => (0x1B, 0x7D), // XXX ] }
            Keycode::Return => (0x1C, 0x0D),
            // 0x1D = CTRL but cant be read as its a modifier
            Keycode::A => (0x1E, 0x41),
            Keycode::S => (0x1F, 0x53),
            Keycode::D => (0x20, 0x44),
            Keycode::F => (0x21, 0x46),
            Keycode::G => (0x22, 0x47),
            Keycode::H => (0x23, 0x48),
            Keycode::J => (0x24, 0x4A),
            Keycode::K => (0x25, 0x4B),
            Keycode::L => (0x26, 0x4C),
            Keycode::Colon => (0x27, 0x3A), // XXX ; :
            Keycode::Quote => (0x28, 0x22), // XXX ' "
            Keycode::Caret => (0x29, 0x7E), // XXX ` ~
            // 0x2A = Left Shift
            Keycode::Backslash => (0x2B, 0x7C), // XXX \ |
            Keycode::Z => (0x2C, 0x5A),
            Keycode::X => (0x2D, 0x58),
            Keycode::C => (0x2E, 0x43),
            Keycode::V => (0x2F, 0x56),
            Keycode::B => (0x30, 0x42),
            Keycode::N => (0x31, 0x4E),
            Keycode::M => (0x32, 0x4D),
            Keycode::Comma => (0x33, 0x3C), // XXX , <
            Keycode::Period => (0x34, 0x3E), // XXX . >
            Keycode::Slash => (0x35, 0x3F),  // XXX / ?
            // 0x36 = Right Shift
            Keycode::Asterisk => (0x37, 0x2A),
            // 0x38 = Alt
            Keycode::Space => (0x39, 0x20),
            // 0x3A = Caps Lock
            Keycode::F1 => (0x54, 0x00),
            Keycode::F2 => (0x55, 0x00),
            Keycode::F3 => (0x56, 0x00),
            Keycode::F4 => (0x57, 0x00),
            Keycode::F5 => (0x58, 0x00),
            Keycode::F6 => (0x59, 0x00),
            Keycode::F7 => (0x5A, 0x00),
            Keycode::F8 => (0x5B, 0x00),
            Keycode::F9 => (0x5C, 0x00),
            Keycode::F10 => (0x5D, 0x00),
            // 0x45 = Num Lock
            // 0x46 = Scroll Lock
            Keycode::Home => (0x47, 0x37),
            Keycode::Up => (0x48, 0x38),
            Keycode::PageUp => (0x49, 0x39),
            Keycode::KpMinus => (0x4A, 0x2D), // XXX numeric keypad minus
            Keycode::Left => (0x4B, 0x34),
            Keycode::KpClearEntry => (0x4C, 0x35), // XXX center numeric keyb
            Keycode::Right => (0x4D, 0x36),
            Keycode::KpPlus => (0x4E, 0x2B), // XXX numeric keypad plus
            Keycode::End => (0x4F, 0x31),
            Keycode::Down => (0x50, 0x32),
            Keycode::PageDown => (0x51, 0x33),
            Keycode::Insert => (0x52, 0x30),
            Keycode::Delete => (0x53, 0x2E),
            _ => {
                println!("unhandled SHIFT keycode mapping for {:#?}", self.keycode);
                (0, 0)
            }
        }
    }

    pub fn to_std_ctrl(&self) -> (u8, u8) {
        match self.keycode {
            _ => {
                println!("unhandled CTRL keycode mapping for {:#?}", self.keycode);
                (0, 0)
            }
        }
    }

    pub fn to_std_alt(&self) -> (u8, u8) {
        match self.keycode {
            _ => {
                println!("unhandled ALT keycode mapping for {:#?}", self.keycode);
                (0, 0)
            }
        }
    }
}

// returns scancode, ascii
fn map_sdl_to_dos_standard_codes(keypress: &Keypress) -> (u8, u8) {
    match keypress.keycode {
        // misc mappings
        Keycode::LGui => (0, 0),
        Keycode::LShift => (0, 0),
        Keycode::RShift => (0, 0),
        _ => {
            if keypress.modifier == Mod::LSHIFTMOD || keypress.modifier == Mod::RSHIFTMOD {
                keypress.to_std_shift()
            } else if keypress.modifier == Mod::LCTRLMOD || keypress.modifier == Mod::RCTRLMOD {
                keypress.to_std_ctrl()
            } else if keypress.modifier == Mod::LALTMOD || keypress.modifier == Mod::RALTMOD {
                keypress.to_std_alt()
            } else {
                keypress.to_std_normal()
            }
        }
    }
}
