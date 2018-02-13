use gpu::GPU;
use memory::mmu::MMU;
use pit::PIT;
use pic::PIC;
use bios::BIOS;

pub struct Hardware {
    pub gpu: GPU,
    pub mmu: MMU,
    pub bios: BIOS,
    pub pit: PIT,
    pub pic: PIC,
    pub pic2: PIC, // secondary pic
}

impl Hardware {
    pub fn new() -> Self {
        let mut mmu = MMU::new();
        let mut gpu = GPU::new();
        let mut bios = BIOS::new();
        bios.init(&mut mmu);
        gpu.init(&mut mmu);
        Hardware {
            mmu: mmu,
            gpu: gpu,
            bios: bios,
            pit: PIT::new(),
            pic: PIC::new(),
            pic2: PIC::new(),
        }
    }
}
