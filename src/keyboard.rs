// TODO later: dont depend on sdl2 in the core crate (process events with something else?)
use sdl2::keyboard::Keycode;

#[derive(Clone)]
pub struct Keyboard {
    /// as returned by sdl2
    keycodes: Vec<sdl2::keyboard::Keycode>,
}

impl Keyboard {
    pub fn default() -> Self {
        Keyboard {
            keycodes: Vec::new(),
        }
    }

    pub fn has_queued_presses(&self) -> bool {
        self.keycodes.len() > 0
    }

    pub fn add_keycode(&mut self, keycode: Keycode) {
        self.keycodes.push(keycode);
    }

    fn consume_keycode(&mut self) -> Keycode {
        self.keycodes.pop().unwrap()
    }

    fn peek_keycode(&self) -> Keycode {
        self.keycodes[0]
    }

    // used by int 0x16 function 0x00
    pub fn consume_dos_standard_scancode_and_ascii(&mut self) -> (u8, u8) {
        let (ah, al, keycode) = self.peek_dos_standard_scancode_and_ascii();
        if ah != 0 {
            self.keycodes.remove_item(&keycode);
        }
        (ah, al)
    }

    // used by int 0x16 function 0x01
    pub fn peek_dos_standard_scancode_and_ascii(&self) -> (u8, u8, Keycode) {
        if !self.has_queued_presses() {
            return (0, 0, Keycode::Num0); // XXX null keycode
        }

        let keycode = self.peek_keycode();
        let (ah, al) = self.map_sdl_to_standard_scancode_and_ascii(keycode);

        (ah, al, keycode)
    }

    // returns scancode, ascii
    fn map_sdl_to_standard_scancode_and_ascii(&self, keycode: Keycode) -> (u8, u8) {

        // normal results. TODO results with SHIFT down, CTRL, ALT

        // https://sites.google.com/site/pcdosretro/scancodes
        match keycode {
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
            Keycode::KpEnter => (0x1C, 0x0D),
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

            // misc mappings
            Keycode::LGui => (0, 0),
            _ => {
                println!("XXX unhandled keycode mapping for {:#?}", keycode);

                (0, 0)
            }
        }
    }
}
