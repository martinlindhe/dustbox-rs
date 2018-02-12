use gpu::GPU;
use gpu::font::load_fonts;
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
        load_fonts(&mut mmu);
        Hardware {
            mmu: mmu,
            gpu: GPU::new(),
            pit: PIT::new(),
            pic: PIC::new(),
            pic2: PIC::new(),
        }
    }
}
