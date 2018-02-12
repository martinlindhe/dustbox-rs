use gpu::GPU;
use memory::mmu::MMU;
use pit::PIT;
use pic::PIC;

pub struct Hardware {
    pub gpu: GPU,
    pub mmu: MMU,
    pub pit: PIT,
    pub pic: PIC,
    pub pic2: PIC, // secondary pic
}

impl Hardware {
    pub fn new() -> Self {
        let mut mmu = MMU::new();
        let mut gpu = GPU::new();
        gpu.init_rom_memory(&mut mmu);
        Hardware {
            mmu: mmu,
            gpu: gpu,
            pit: PIT::new(),
            pic: PIC::new(),
            pic2: PIC::new(),
        }
    }
}
