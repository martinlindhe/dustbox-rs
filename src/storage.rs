use crate::cpu::{CPU, R};
use crate::machine::Component;
use crate::memory::MMU;

// mass storage (disk, floppy)
pub struct Storage {
}

impl Component for Storage {
    fn int(&mut self, int: u8, cpu: &mut CPU, _mmu: &mut MMU) -> bool {
        if int != 0x13 {
            return false;
        }
        match cpu.get_r8(R::AH) {
            0x00 => {
                // DISK - RESET DISK DRIVES
                // DL = drive (if bit 7 is set both hard disks and floppy disks reset)
                println!("XXX DISK - RESET DISK SYSTEM, dl={:02X}", cpu.get_r8(R::DL))
                // Return:
                // AH = status (see #00234)
                // CF clear if successful (returned AH=00h)
                // CF set on error
            }
            _ => return false
        }

        true
    }
}

impl Storage {
    pub fn default() -> Self {
        Self {
        }
    }
}
