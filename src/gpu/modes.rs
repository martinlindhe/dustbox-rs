// lists copied from dosbox-x, int10_modes.cpp

#[derive(Clone, Debug, PartialEq)]
pub enum GFXMode {
    // called VGAModes in dosbox-x
    TEXT,
    CGA2,
    CGA4,
    EGA,
    VGA,
    TANDY16,
    LIN4,
    LIN8,
    LIN15,
    LIN16,
    LIN24,
    LIN32,
}

impl Default for GFXMode {
    fn default() -> Self { GFXMode::TEXT }
}

#[derive(Clone, Default)]
pub struct VideoModeBlock {
    pub mode: u16,
    pub kind: GFXMode,      // called "type" in dosbox-x
    pub swidth: u32,
    pub sheight: u32,
    pub twidth: usize,
    pub theight: usize,
    pub cwidth: usize,
    pub cheight: usize,
    pub ptotal: usize,
    pub pstart: u32,
    pub plength: usize,
    pub htotal: u32,        // Horizontal Total
    pub vtotal: usize,      // Vertical Total
    pub hdispend: usize,    // Horizontal Display End
    pub vdispend: usize,    // Vertical Display End
    pub special: SpecialMode,
}

#[derive(Clone, PartialEq)]
pub struct SpecialMode {
    pub EGA_HALF_CLOCK: bool,
    pub DOUBLESCAN: bool,
    pub VGA_PIXEL_DOUBLE: bool,
    pub S3_PIXEL_DOUBLE: bool,
    pub REPEAT1: bool,
    pub CGA_SYNCDOUBLE: bool,
}

impl Default for SpecialMode {
    fn default() -> Self {
        SpecialMode {
            EGA_HALF_CLOCK: false,
            DOUBLESCAN: false,
            VGA_PIXEL_DOUBLE: false,
            S3_PIXEL_DOUBLE: false,
            REPEAT1: false,
            CGA_SYNCDOUBLE: false,
        }
    }
}

pub fn ega_mode_block() -> [VideoModeBlock; 12] {[
    VideoModeBlock{mode: 0x000, kind: GFXMode::TEXT, swidth: 320, sheight: 350, twidth: 40, theight: 25, cwidth: 8, cheight: 14, ptotal: 8, pstart: 0xB8000, plength: 0x0800, htotal: 50,  vtotal: 366, hdispend: 40, vdispend: 350, special: SpecialMode{EGA_HALF_CLOCK: true, ..Default::default()}},
    VideoModeBlock{mode: 0x001, kind: GFXMode::TEXT, swidth: 320, sheight: 350, twidth: 40, theight: 25, cwidth: 8, cheight: 14, ptotal: 8, pstart: 0xB8000, plength: 0x0800, htotal: 50,  vtotal: 366, hdispend: 40, vdispend: 350, special: SpecialMode{EGA_HALF_CLOCK: true, ..Default::default()}},
    VideoModeBlock{mode: 0x002, kind: GFXMode::TEXT, swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 8, pstart: 0xB8000, plength: 0x1000, htotal: 96,  vtotal: 366, hdispend: 80, vdispend: 350, special: SpecialMode::default()},
    VideoModeBlock{mode: 0x003, kind: GFXMode::TEXT, swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 8, pstart: 0xB8000, plength: 0x1000, htotal: 96,  vtotal: 366, hdispend: 80, vdispend: 350, special: Default::default()},
    VideoModeBlock{mode: 0x004, kind: GFXMode::CGA4, swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 60,  vtotal: 262, hdispend: 40, vdispend: 200, special: SpecialMode{EGA_HALF_CLOCK: true, REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x005, kind: GFXMode::CGA4, swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 60,  vtotal: 262, hdispend: 40, vdispend: 200, special: SpecialMode{EGA_HALF_CLOCK: true, REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x006, kind: GFXMode::CGA2, swidth: 640, sheight: 200, twidth: 80, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 120, vtotal: 262, hdispend: 80, vdispend: 200, special: SpecialMode{REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x007, kind: GFXMode::TEXT, swidth: 720, sheight: 350, twidth: 80, theight: 25, cwidth: 9, cheight: 14, ptotal: 8, pstart: 0xB0000, plength: 0x1000, htotal: 120, vtotal: 440, hdispend: 80, vdispend: 350, special: Default::default()},

    VideoModeBlock{mode: 0x00D, kind: GFXMode::EGA,  swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 8, pstart: 0xA0000, plength: 0x2000, htotal: 60,  vtotal: 262, hdispend: 40, vdispend: 200, special: SpecialMode{EGA_HALF_CLOCK: true, ..Default::default()}},
    VideoModeBlock{mode: 0x00E, kind: GFXMode::EGA,  swidth: 640, sheight: 200, twidth: 80, theight: 25, cwidth: 8, cheight: 8,  ptotal: 4, pstart: 0xA0000, plength: 0x4000, htotal: 120, vtotal: 262, hdispend: 80, vdispend: 200, special: Default::default()},
    VideoModeBlock{mode: 0x00F, kind: GFXMode::EGA,  swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 2, pstart: 0xA0000, plength: 0x8000, htotal: 96,  vtotal: 366, hdispend: 80, vdispend: 350, special: Default::default()},
    VideoModeBlock{mode: 0x010, kind: GFXMode::EGA,  swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 2, pstart: 0xA0000, plength: 0x8000, htotal: 96,  vtotal: 366, hdispend: 80, vdispend: 350, special: Default::default()},
]}

pub fn vga_mode_block() -> [VideoModeBlock; 15] {[
    VideoModeBlock{mode: 0x000, kind: GFXMode::TEXT, swidth: 360, sheight: 400, twidth: 40, theight: 25, cwidth: 9, cheight: 16, ptotal: 8, pstart: 0xB8000, plength: 0x0800, htotal: 50,  vtotal: 449, hdispend: 40, vdispend: 400, special: SpecialMode{EGA_HALF_CLOCK: true, ..Default::default()}},
    VideoModeBlock{mode: 0x001, kind: GFXMode::TEXT, swidth: 360, sheight: 400, twidth: 40, theight: 25, cwidth: 9, cheight: 16, ptotal: 8, pstart: 0xB8000, plength: 0x0800, htotal: 50,  vtotal: 449, hdispend: 40, vdispend: 400, special: SpecialMode{EGA_HALF_CLOCK: true, ..Default::default()}},
    VideoModeBlock{mode: 0x002, kind: GFXMode::TEXT, swidth: 720, sheight: 400, twidth: 80, theight: 25, cwidth: 9, cheight: 16, ptotal: 8, pstart: 0xB8000, plength: 0x1000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: Default::default()},
    VideoModeBlock{mode: 0x003, kind: GFXMode::TEXT, swidth: 720, sheight: 400, twidth: 80, theight: 25, cwidth: 9, cheight: 16, ptotal: 8, pstart: 0xB8000, plength: 0x1000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: Default::default()},
    VideoModeBlock{mode: 0x004, kind: GFXMode::CGA4, swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 50,  vtotal: 449, hdispend: 40, vdispend: 400, special: SpecialMode{EGA_HALF_CLOCK: true, DOUBLESCAN: true, REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x005, kind: GFXMode::CGA4, swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 50,  vtotal: 449, hdispend: 40, vdispend: 400, special: SpecialMode{EGA_HALF_CLOCK: true, DOUBLESCAN: true, REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x006, kind: GFXMode::CGA2, swidth: 640, sheight: 200, twidth: 80, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xB8000, plength: 0x4000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: SpecialMode{DOUBLESCAN: true, REPEAT1: true, ..Default::default()}},
    VideoModeBlock{mode: 0x007, kind: GFXMode::TEXT, swidth: 720, sheight: 400, twidth: 80, theight: 25, cwidth: 9, cheight: 16, ptotal: 8, pstart: 0xB0000, plength: 0x1000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: Default::default()},

    VideoModeBlock{mode: 0x00D, kind: GFXMode::EGA,  swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 8, pstart: 0xA0000, plength: 0x2000, htotal: 50,  vtotal: 449, hdispend: 40, vdispend: 400, special: SpecialMode{EGA_HALF_CLOCK: true, DOUBLESCAN: true, ..Default::default()}},
    VideoModeBlock{mode: 0x00E, kind: GFXMode::EGA,  swidth: 640, sheight: 200, twidth: 80, theight: 25, cwidth: 8, cheight: 8,  ptotal: 4, pstart: 0xA0000, plength: 0x4000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: SpecialMode{DOUBLESCAN: true, ..Default::default()}},
    VideoModeBlock{mode: 0x00F, kind: GFXMode::EGA,  swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 2, pstart: 0xA0000, plength: 0x8000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 350, special: Default::default()},
    VideoModeBlock{mode: 0x010, kind: GFXMode::EGA,  swidth: 640, sheight: 350, twidth: 80, theight: 25, cwidth: 8, cheight: 14, ptotal: 2, pstart: 0xA0000, plength: 0x8000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 350, special: Default::default()},
    VideoModeBlock{mode: 0x011, kind: GFXMode::EGA,  swidth: 640, sheight: 480, twidth: 80, theight: 30, cwidth: 8, cheight: 16, ptotal: 1, pstart: 0xA0000, plength: 0xA000, htotal: 100, vtotal: 525, hdispend: 80, vdispend: 480, special: Default::default()},
    VideoModeBlock{mode: 0x012, kind: GFXMode::EGA,  swidth: 640, sheight: 480, twidth: 80, theight: 30, cwidth: 8, cheight: 16, ptotal: 1, pstart: 0xA0000, plength: 0xA000, htotal: 100, vtotal: 525, hdispend: 80, vdispend: 480, special: Default::default()},
    VideoModeBlock{mode: 0x013, kind: GFXMode::VGA,  swidth: 320, sheight: 200, twidth: 40, theight: 25, cwidth: 8, cheight: 8,  ptotal: 1, pstart: 0xA0000, plength: 0x2000, htotal: 100, vtotal: 449, hdispend: 80, vdispend: 400, special: SpecialMode{REPEAT1: true, ..Default::default()}},
    /*
    {0x054  ,M_TEXT   ,1056,344, 132,43, 8,  8, 1 ,0xB8000 ,0x4000, 160, 449, 132,344, 0   },
    {0x055  ,M_TEXT   ,1056,400, 132,25, 8, 16, 1 ,0xB8000 ,0x2000, 160, 449, 132,400, 0   },

    /* Alias of mode 101 */
    {0x069  ,M_LIN8   ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 ,0},
    /* Alias of mode 102 */
    {0x06A  ,M_LIN4   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,128 ,663 ,100,600 ,0},

    /* Follow vesa 1.2 for first 0x20 */
    {0x100  ,M_LIN8   ,640 ,400 ,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 ,0   },
    {0x101  ,M_LIN8   ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 , _VGA_PIXEL_DOUBLE },
    {0x102  ,M_LIN4   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,628 ,100,600 ,0},
    {0x103  ,M_LIN8   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,628 ,100,600 ,0},
    {0x104  ,M_LIN4   ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,806 ,128,768 ,0},
    {0x105  ,M_LIN8   ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,806 ,128,768 ,0},
    {0x106  ,M_LIN4   ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,212 ,1066,160,1024,0},
    {0x107  ,M_LIN8   ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,212 ,1066,160,1024,0},

    /* VESA text modes */ 
    {0x108  ,M_TEXT   ,640 ,480,  80,60, 8,  8 ,2 ,0xB8000 ,0x4000, 100 ,525 ,80 ,480 ,0   },
    {0x109  ,M_TEXT   ,1056,400, 132,25, 8, 16, 1 ,0xB8000 ,0x2000, 160, 449, 132,400, 0   },
    {0x10A  ,M_TEXT   ,1056,688, 132,43, 8,  8, 1 ,0xB8000 ,0x4000, 160, 449, 132,344, 0   },
    {0x10B  ,M_TEXT   ,1056,400, 132,50, 8,  8, 1 ,0xB8000 ,0x4000, 160, 449, 132,400, 0   },
    {0x10C  ,M_TEXT   ,1056,480, 132,60, 8,  8, 2 ,0xB8000 ,0x4000, 160, 531, 132,480, 0   },

    /* VESA higher color modes.
    * Note v1.2 of the VESA BIOS extensions explicitly states modes 0x10F, 0x112, 0x115, 0x118 are 8:8:8 (24-bit) not 8:8:8:8 (32-bit).
    * This also fixes COMA "Parhaat" 1997 demo, by offering a true 24bpp mode so that it doesn't try to draw 24bpp on a 32bpp VESA linear framebuffer.
    * NTS: The 24bpp modes listed here will not be available to the DOS game/demo if the user says that the VBE 1.2 modes are 32bpp,
    *      instead the redefinitions in the next block will apply to allow M_LIN32. To use the 24bpp modes here, you must set 'vesa vbe 1.2 modes are 32bpp=false' */
    {0x10D  ,M_LIN15  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , _DOUBLESCAN },
    {0x10E  ,M_LIN16  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , _DOUBLESCAN },
    {0x10F  ,M_LIN24  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,50  ,449 ,40 ,400 , _DOUBLESCAN },
    {0x110  ,M_LIN15  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,525 ,160,480 ,0   },
    {0x111  ,M_LIN16  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,525 ,160,480 ,0   },
    {0x112  ,M_LIN24  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 ,0   },
    {0x113  ,M_LIN15  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,628 ,200,600 ,0   },
    {0x114  ,M_LIN16  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,628 ,200,600 ,0   },
    {0x115  ,M_LIN24  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,628 ,100,600 ,0   },
    {0x116  ,M_LIN15  ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,336 ,806 ,256,768 ,0},
    {0x117  ,M_LIN16  ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,336 ,806 ,256,768 ,0},
    {0x118  ,M_LIN24  ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,806 ,128,768 ,0},

    /* But of course... there are other demos that assume mode 0x10F is 32bpp!
    * So we have another definition of those modes that overlaps some of the same mode numbers above.
    * This allows "Phenomena" demo to use 32bpp 320x200 mode if you set 'vesa vbe 1.2 modes are 32bpp=true'.
    * The code will allow either this block's mode 0x10F (LIN32), or the previous block's mode 0x10F (LIN24), but not both. */
    {0x10F  ,M_LIN32  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,50  ,449 ,40 ,400 , _DOUBLESCAN },
    {0x112  ,M_LIN32  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 ,0   },
    {0x115  ,M_LIN32  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,628 ,100,600 ,0   },
    {0x118  ,M_LIN32  ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,806 ,128,768 ,0},

    /* RGBX 8:8:8:8 modes. These were once the M_LIN32 modes DOSBox mapped to 0x10F-0x11B prior to implementing M_LIN24. */
    {0x210  ,M_LIN32  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,50  ,449 ,40 ,400 , _DOUBLESCAN },
    {0x211  ,M_LIN32  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 ,0   },
    {0x212  ,M_LIN32  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,628 ,100,600 ,0   },
    {0x214  ,M_LIN32  ,1024,768 ,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,806 ,128,768 ,0},

    /* those should be interlaced but ok */
    {0x119  ,M_LIN15  ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,424 ,1066,320,1024,0},
    {0x11A  ,M_LIN16  ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,424 ,1066,320,1024,0},

    {0x11C  ,M_LIN8   ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x10000,100 ,449 ,80 ,350 ,0},
    // special mode for Birth demo by Incognita
    {0x11D  ,M_LIN15  ,640 ,350 ,80 ,25 ,8 ,14 ,1 ,0xA0000 ,0x10000,200 ,449 ,160,350 ,0   },
    {0x11F  ,M_LIN16  ,640 ,350 ,80 ,25 ,8 ,14 ,1 ,0xA0000 ,0x10000,200 ,449 ,160,350 ,0   },
    {0x120  ,M_LIN8   ,1600,1200,200,75 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1240,200,1200,0},
    {0x142  ,M_LIN32  ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x10000 ,100 ,449 ,80 ,350 ,0},

    // FIXME: Find an old S3 Trio and dump the VESA modelist, then arrange this modelist to match
    {0x150  ,M_LIN8   ,320 ,480 ,40 ,60 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 , _S3_PIXEL_DOUBLE  },
    {0x151  ,M_LIN8   ,320 ,240 ,40 ,30 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 , _S3_PIXEL_DOUBLE | _DOUBLESCAN },
    {0x152  ,M_LIN8   ,320 ,400 ,40 ,50 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , _S3_PIXEL_DOUBLE  },
    // For S3 Trio emulation this mode must exist as mode 0x153 else RealTech "Countdown" will crash
    // if you select VGA 320x200 with S3 acceleration.
    {0x153  ,M_LIN8   ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , _S3_PIXEL_DOUBLE | _DOUBLESCAN },

    {0x160  ,M_LIN15  ,320 ,240 ,40 ,30 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 , 80 ,480 , _DOUBLESCAN },
    {0x161  ,M_LIN15  ,320 ,400 ,40 ,50 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 , 80 ,400 ,0 },
    {0x162  ,M_LIN15  ,320 ,480 ,40 ,60 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 , 80 ,480 ,0 },
    {0x165  ,M_LIN15  ,640 ,400 ,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,449 ,160 ,400 ,0   },

    // hack: 320x480x256-color alias for Habitual demo. doing this removes the need to run S3VBE20.EXE before running the demo.
    //       the reason it has to be this particular video mode is because HABITUAL.EXE does not query modes, it simply assumes
    //       that mode 0x166 is this particular mode and errors out if it can't set it.
    {0x166  ,M_LIN8   ,320 ,480 ,40 ,60 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 , _S3_PIXEL_DOUBLE  },

    {0x170  ,M_LIN16  ,320 ,240 ,40 ,30 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 , 80 ,480 , _DOUBLESCAN },
    {0x171  ,M_LIN16  ,320 ,400 ,40 ,50 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 , 80 ,400 ,0 },
    {0x172  ,M_LIN16  ,320 ,480 ,40 ,60 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,525 , 80 ,480 ,0 },
    {0x175  ,M_LIN16  ,640 ,400 ,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,449 ,160 ,400 ,0   },

    {0x190  ,M_LIN32  ,320 ,240 ,40 ,30 ,8 ,8  ,1 ,0xA0000 ,0x10000, 50 ,525 ,40 ,480 , _DOUBLESCAN },
    {0x191  ,M_LIN32  ,320 ,400 ,40 ,50 ,8 ,8  ,1 ,0xA0000 ,0x10000, 50 ,449 ,40 ,400 ,0 },
    {0x192  ,M_LIN32  ,320 ,480 ,40 ,60 ,8 ,8  ,1 ,0xA0000 ,0x10000, 50 ,525 ,40 ,480 ,0 },

    // S3 specific modes
    {0x207  ,M_LIN8,1152,864,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,182 ,948 ,144,864 ,0},
    {0x209  ,M_LIN15,1152,864,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,364 ,948 ,288,864 ,0},
    {0x20A  ,M_LIN16,1152,864,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,364 ,948 ,288,864 ,0},
    {0x20B  ,M_LIN32,1152, 864,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,182 ,948 ,144,864 ,0},
    {0x213  ,M_LIN32   ,640 ,400,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 ,0},

    // Some custom modes

    // 720x480 3:2 modes
    {0x21B  ,M_LIN4   ,720 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0},
    {0x21C  ,M_LIN8   ,720 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0},
    {0x21D  ,M_LIN15  ,720 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,525 ,212 ,480 ,0  },
    {0x21E  ,M_LIN16  ,720 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,525 ,212 ,480 ,0  },
    {0x21F  ,M_LIN32  ,720 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0  },

    // 848x480 16:9 modes
    {0x220  ,M_LIN4   ,848 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0},
    {0x221  ,M_LIN8   ,848 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0},
    {0x222  ,M_LIN15  ,848 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,525 ,212 ,480 ,0  },
    {0x223  ,M_LIN16  ,848 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,525 ,212 ,480 ,0  },
    {0x224  ,M_LIN32  ,848 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,132 ,525 ,106 ,480 ,0  },

    // 1280x800 8:5 modes
    {0x225  ,M_LIN4   ,1280,800 ,160,50 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,880 ,160 ,800 ,0},
    {0x226  ,M_LIN8   ,1280,800 ,160,50 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,880 ,160 ,800 ,0},
    {0x227  ,M_LIN15  ,1280,800 ,160,50 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,880 ,320 ,800 ,0  },
    {0x228  ,M_LIN16  ,1280,800 ,160,50 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,880 ,320 ,800 ,0  },
    {0x229  ,M_LIN32  ,1280,800 ,160,50 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,880 ,160 ,800 ,0  },

    // 1280x960 4:3 modes
    {0x22a  ,M_LIN4   ,1280,960 ,160,60 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1020,160 ,960 ,0},
    {0x22b  ,M_LIN8   ,1280,960 ,160,60 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1020,160 ,960 ,0},
    {0x22c  ,M_LIN15  ,1280,960 ,160,60 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,1020,320 ,960 ,0  },
    {0x22d  ,M_LIN16  ,1280,960 ,160,60 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,1020,320 ,960 ,0  },
    {0x22e  ,M_LIN32  ,1280,960 ,160,60 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1020,160 ,960 ,0  },

    // 1280x1024 5:4 rest
    {0x22f  ,M_LIN32  ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0x10000,212 ,1066,160,1024,0},

    // 1400x1050 4:3 - 4bpp is no good xD
    {0x22b  ,M_LIN4   ,1400,1050,175,66 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 ,1100,175 ,1050,0},
    {0x230  ,M_LIN8   ,1400,1050,175,66 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 ,1100,175 ,1050,0},
    {0x231  ,M_LIN15  ,1400,1050,175,66 ,8 ,16 ,1 ,0xA0000 ,0x10000,440 ,1100,350 ,1050,0  },
    {0x232  ,M_LIN16  ,1400,1050,175,66 ,8 ,16 ,1 ,0xA0000 ,0x10000,440 ,1100,350 ,1050,0  },
    {0x233  ,M_LIN32  ,1400,1050,175,66 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 ,1100,175 ,1050,0  },

    // 1440x900 8:5 modes
    {0x234  ,M_LIN4   ,1440, 900,180,56 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 , 980,180 , 900,0  },
    {0x235  ,M_LIN8   ,1440, 900,180,56 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 , 980,180 , 900,0  },
    {0x236  ,M_LIN15  ,1440, 900,180,56 ,8 ,16 ,1 ,0xA0000 ,0x10000,440 , 980,360 , 900,0  },
    {0x237  ,M_LIN16  ,1440, 900,180,56 ,8 ,16 ,1 ,0xA0000 ,0x10000,440 , 980,360 , 900,0  },
    {0x238  ,M_LIN32  ,1440, 900,180,56 ,8 ,16 ,1 ,0xA0000 ,0x10000,220 , 980,180 , 900,0  },

    // 1600x1200 4:3 rest - 32bpp needs more than 4 megs
    {0x239  ,M_LIN4   ,1600,1200,200,75 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1240,200, 1200,0},
    {0x23a  ,M_LIN15  ,1600,1200,200,75 ,8 ,16 ,1 ,0xA0000 ,0x10000,500 ,1240,400 ,1200,0},
    {0x23b  ,M_LIN16  ,1600,1200,200,75 ,8 ,16 ,1 ,0xA0000 ,0x10000,500 ,1240,400 ,1200,0},
    {0x23c  ,M_LIN32  ,1600,1200,200,75 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1240,200 ,1200,0},

    // 1280x720 16:9 modes
    {0x23D  ,M_LIN4   ,1280,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,176 ,792 ,160 ,720 ,0},
    {0x23E  ,M_LIN8   ,1280,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,176 ,792 ,160 ,720 ,0},
    {0x23F  ,M_LIN15  ,1280,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,352 ,792 ,320 ,720 ,0  },
    {0x240  ,M_LIN16  ,1280,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,352 ,792 ,320 ,720 ,0  },
    {0x241  ,M_LIN32  ,1280,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,176 ,792 ,160 ,720 ,0  },

    // 1920x1080 16:9 modes
    {0x242  ,M_LIN4   ,1920,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1188,240 ,1080,0},
    {0x243  ,M_LIN8   ,1920,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1188,240 ,1080,0},
    {0x244  ,M_LIN15  ,1920,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,528 ,1188,480 ,1080,0  },
    {0x245  ,M_LIN16  ,1920,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,528 ,1188,480 ,1080,0  },
    {0x246  ,M_LIN32  ,1920,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,1188,240 ,1080,0  },

    // 960x720 4:3 modes
    {0x247  ,M_LIN4   ,960,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,144 ,792 ,120 ,720 ,0},
    {0x248  ,M_LIN8   ,960,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,144 ,792 ,120 ,720 ,0},
    {0x249  ,M_LIN15  ,960,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,288 ,792 ,240 ,720 ,0  },
    {0x24A  ,M_LIN16  ,960,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,288 ,792 ,240 ,720 ,0  },
    {0x24B  ,M_LIN32  ,960,720 ,160,45 ,8 ,16 ,1 ,0xA0000 ,0x10000,144 ,792 ,120 ,720 ,0  },

    // 1440x1080 16:9 modes
    {0x24C  ,M_LIN4   ,1440,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1188,180 ,1080,0},
    {0x24D  ,M_LIN8   ,1440,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1188,180 ,1080,0},
    {0x24E  ,M_LIN15  ,1440,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,1188,360 ,1080,0  },
    {0x24F  ,M_LIN16  ,1440,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,400 ,1188,360 ,1080,0  },
    {0x2F0  ,M_LIN32  ,1440,1080,240,67 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,1188,180 ,1080,0  },

    {0xFFFF  ,M_ERROR  ,0   ,0   ,0  ,0  ,0 ,0  ,0 ,0x00000 ,0x0000 ,0   ,0   ,0  ,0   ,0 },
*/
]}

/*
VideoModeBlock ModeList_VGA_Text_200lines[]={
/* mode  ,type     ,sw  ,sh  ,tw ,th ,cw,ch ,pt,pstart  ,plength,htot,vtot,hde,vde special flags */
{0x000  ,M_TEXT   ,320 ,200 ,40 ,25 ,8 , 8 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK | _DOUBLESCAN},
{0x001  ,M_TEXT   ,320 ,200 ,40 ,25 ,8 , 8 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK | _DOUBLESCAN},
{0x002  ,M_TEXT   ,640 ,200 ,80 ,25 ,8 , 8 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN },
{0x003  ,M_TEXT   ,640 ,200 ,80 ,25 ,8 , 8 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN }
};

VideoModeBlock ModeList_VGA_Text_350lines[]={
/* mode  ,type     ,sw  ,sh  ,tw ,th ,cw,ch ,pt,pstart  ,plength,htot,vtot,hde,vde special flags */
{0x000  ,M_TEXT   ,320 ,350 ,40 ,25 ,8 ,14 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,350 ,_EGA_HALF_CLOCK},
{0x001  ,M_TEXT   ,320 ,350 ,40 ,25 ,8 ,14 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,350 ,_EGA_HALF_CLOCK},
{0x002  ,M_TEXT   ,640 ,350 ,80 ,25 ,8 ,14 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,350 ,0},
{0x003  ,M_TEXT   ,640 ,350 ,80 ,25 ,8 ,14 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,350 ,0}
};

VideoModeBlock ModeList_VGA_Tseng[]={
/* mode  ,type     ,sw  ,sh  ,tw ,th ,cw,ch ,pt,pstart  ,plength,htot,vtot,hde,vde special flags */
{0x000  ,M_TEXT   ,360 ,400 ,40 ,25 ,9 ,16 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK},
{0x001  ,M_TEXT   ,360 ,400 ,40 ,25 ,9 ,16 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK},
{0x002  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,0},
{0x003  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,0},
{0x004  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN | _REPEAT1},
{0x005  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN | _REPEAT1},
{0x006  ,M_CGA2   ,640 ,200 ,80 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN | _REPEAT1},
{0x007  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB0000 ,0x1000 ,100 ,449 ,80 ,400 ,0},

{0x00D  ,M_EGA    ,320 ,200 ,40 ,25 ,8 ,8  ,8 ,0xA0000 ,0x2000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN},
{0x00E  ,M_EGA    ,640 ,200 ,80 ,25 ,8 ,8  ,4 ,0xA0000 ,0x4000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN },
{0x00F  ,M_EGA    ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x8000 ,100 ,449 ,80 ,350 ,0},
{0x010  ,M_EGA    ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x8000 ,100 ,449 ,80 ,350 ,0},
{0x011  ,M_EGA    ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0xA000 ,100 ,525 ,80 ,480 ,0},
{0x012  ,M_EGA    ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0xA000 ,100 ,525 ,80 ,480 ,0},
{0x013  ,M_VGA    ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x2000 ,100 ,449 ,80 ,400 ,_REPEAT1   },

{0x018  ,M_TEXT   ,1056 ,688, 132,44, 8, 8, 1 ,0xB0000 ,0x4000, 192, 800, 132, 704, 0 },
{0x019  ,M_TEXT   ,1056 ,400, 132,25, 8, 16,1 ,0xB0000 ,0x2000, 192, 449, 132, 400, 0 },
{0x01A  ,M_TEXT   ,1056 ,400, 132,28, 8, 16,1 ,0xB0000 ,0x2000, 192, 449, 132, 448, 0 },
{0x022  ,M_TEXT   ,1056 ,688, 132,44, 8, 8, 1 ,0xB8000 ,0x4000, 192, 800, 132, 704, 0 },
{0x023  ,M_TEXT   ,1056 ,400, 132,25, 8, 16,1 ,0xB8000 ,0x2000, 192, 449, 132, 400, 0 },
{0x024  ,M_TEXT   ,1056 ,400, 132,28, 8, 16,1 ,0xB8000 ,0x2000, 192, 449, 132, 448, 0 },
{0x025  ,M_LIN4   ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0xA000 ,100 ,525 ,80 ,480 , 0 },
{0x029  ,M_LIN4   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0xA000, 128 ,663 ,100,600 , 0 },
{0x02D  ,M_LIN8   ,640 ,350 ,80 ,21 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,350 , 0 },
{0x02E  ,M_LIN8   ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,525 ,80 ,480 , 0 },
{0x02F  ,M_LIN8   ,640 ,400 ,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , 0 },/* ET4000 only */
{0x030  ,M_LIN8   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,128 ,663 ,100,600 , 0 },
{0x036  ,M_LIN4   ,960 , 720,120,45 ,8 ,16 ,1 ,0xA0000 ,0xA000, 120 ,800 ,120,720 , 0 },/* STB only */
{0x037  ,M_LIN4   ,1024, 768,128,48 ,8 ,16 ,1 ,0xA0000 ,0xA000, 128 ,800 ,128,768 , 0 },
{0x038  ,M_LIN8   ,1024 ,768,128,48 ,8 ,16 ,1 ,0xA0000 ,0x10000,168 ,800 ,128,768 , 0 },/* ET4000 only */
{0x03D  ,M_LIN4   ,1280,1024,160,64 ,8 ,16 ,1 ,0xA0000 ,0xA000, 160 ,1152,160,1024, 0 },/* newer ET4000 */
{0x03E  ,M_LIN4   ,1280, 960,160,60 ,8 ,16 ,1 ,0xA0000 ,0xA000, 160 ,1024,160,960 , 0 },/* Definicon only */ 
{0x06A  ,M_LIN4   ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0xA000, 128 ,663 ,100,600 , 0 },/* newer ET4000 */

// Sierra SC1148x Hi-Color DAC modes
{0x213  ,M_LIN15  ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x10000,100 ,449 ,80 ,400 , _VGA_PIXEL_DOUBLE | _DOUBLESCAN },
{0x22D  ,M_LIN15  ,640 ,350 ,80 ,25 ,8 ,14 ,1 ,0xA0000 ,0x10000,200 ,449 ,160,350 , 0 },
{0x22E  ,M_LIN15  ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,525 ,160,480 , 0 },
{0x22F  ,M_LIN15  ,640 ,400 ,80 ,25 ,8 ,16 ,1 ,0xA0000 ,0x10000,200 ,449 ,160,400 , 0 },
{0x230  ,M_LIN15  ,800 ,600 ,100,37 ,8 ,16 ,1 ,0xA0000 ,0x10000,264 ,628 ,200,600 , 0 },

{0xFFFF  ,M_ERROR  ,0   ,0   ,0  ,0  ,0 ,0  ,0 ,0x00000 ,0x0000 ,0   ,0   ,0  ,0   ,0 },
};

VideoModeBlock ModeList_VGA_Paradise[]={
/* mode  ,type     ,sw  ,sh  ,tw ,th ,cw,ch ,pt,pstart  ,plength,htot,vtot,hde,vde special flags */
{0x000  ,M_TEXT   ,360 ,400 ,40 ,25 ,9 ,16 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK},
{0x001  ,M_TEXT   ,360 ,400 ,40 ,25 ,9 ,16 ,8 ,0xB8000 ,0x0800 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK},
{0x002  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,0},
{0x003  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB8000 ,0x1000 ,100 ,449 ,80 ,400 ,0},
{0x004  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN | _REPEAT1},
{0x005  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN | _REPEAT1},
{0x006  ,M_CGA2   ,640 ,200 ,80 ,25 ,8 ,8  ,1 ,0xB8000 ,0x4000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN | _REPEAT1},
{0x007  ,M_TEXT   ,720 ,400 ,80 ,25 ,9 ,16 ,8 ,0xB0000 ,0x1000 ,100 ,449 ,80 ,400 ,0},

{0x00D  ,M_EGA    ,320 ,200 ,40 ,25 ,8 ,8  ,8 ,0xA0000 ,0x2000 ,50  ,449 ,40 ,400 ,_EGA_HALF_CLOCK| _DOUBLESCAN},
{0x00E  ,M_EGA    ,640 ,200 ,80 ,25 ,8 ,8  ,4 ,0xA0000 ,0x4000 ,100 ,449 ,80 ,400 ,_DOUBLESCAN },
{0x00F  ,M_EGA    ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x8000 ,100 ,449 ,80 ,350 ,0},
{0x010  ,M_EGA    ,640 ,350 ,80 ,25 ,8 ,14 ,2 ,0xA0000 ,0x8000 ,100 ,449 ,80 ,350 ,0},
{0x011  ,M_EGA    ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0xA000 ,100 ,525 ,80 ,480 ,0},
{0x012  ,M_EGA    ,640 ,480 ,80 ,30 ,8 ,16 ,1 ,0xA0000 ,0xA000 ,100 ,525 ,80 ,480 ,0},
{0x013  ,M_VGA    ,320 ,200 ,40 ,25 ,8 ,8  ,1 ,0xA0000 ,0x2000 ,100 ,449 ,80 ,400 ,_REPEAT1 },

{0x054  ,M_TEXT   ,1056 ,688, 132,43, 8, 9, 1, 0xB0000, 0x4000, 192, 720, 132,688, 0 },
{0x055  ,M_TEXT   ,1056 ,400, 132,25, 8, 16,1, 0xB0000, 0x2000, 192, 449, 132,400, 0 },
{0x056  ,M_TEXT   ,1056 ,688, 132,43, 8, 9, 1, 0xB0000, 0x4000, 192, 720, 132,688, 0 },
{0x057  ,M_TEXT   ,1056 ,400, 132,25, 8, 16,1, 0xB0000, 0x2000, 192, 449, 132,400, 0 },
{0x058  ,M_LIN4   ,800 , 600, 100,37, 8, 16,1, 0xA0000, 0xA000, 128 ,663 ,100,600, 0 },
{0x05C  ,M_LIN8   ,800 , 600 ,100,37 ,8 ,16,1 ,0xA0000 ,0x10000,128 ,663 ,100,600, 0 },
{0x05D  ,M_LIN4   ,1024, 768, 128,48 ,8, 16,1, 0xA0000, 0x10000,128 ,800 ,128,768 ,0 }, // documented only on C00 upwards
{0x05E  ,M_LIN8   ,640 , 400, 80 ,25, 8, 16,1, 0xA0000, 0x10000,100 ,449 ,80 ,400, 0 },
{0x05F  ,M_LIN8   ,640 , 480, 80 ,30, 8, 16,1, 0xA0000, 0x10000,100 ,525 ,80 ,480, 0 },

{0xFFFF  ,M_ERROR  ,0   ,0   ,0  ,0  ,0 ,0  ,0 ,0x00000 ,0x0000 ,0   ,0   ,0  ,0   ,0 },
};

VideoModeBlock ModeList_OTHER[]={
/* mode  ,type     ,sw  ,sh  ,tw ,th ,cw,ch ,pt,pstart  ,plength,htot,vtot,hde,vde ,special flags */
{0x000  ,M_TEXT   ,320 ,400 ,40 ,25 ,8 ,8  ,8 ,0xB8000 ,0x0800 ,56  ,31  ,40 ,25  ,0   },
{0x001  ,M_TEXT   ,320 ,400 ,40 ,25 ,8 ,8  ,8 ,0xB8000 ,0x0800 ,56  ,31  ,40 ,25  ,0},
{0x002  ,M_TEXT   ,640 ,400 ,80 ,25 ,8 ,8  ,4 ,0xB8000 ,0x1000 ,113 ,31  ,80 ,25  ,0},
{0x003  ,M_TEXT   ,640 ,400 ,80 ,25 ,8 ,8  ,4 ,0xB8000 ,0x1000 ,113 ,31  ,80 ,25  ,0},
{0x004  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,4 ,0xB8000 ,0x0800 ,56  ,127 ,40 ,100 ,0   },
{0x005  ,M_CGA4   ,320 ,200 ,40 ,25 ,8 ,8  ,4 ,0xB8000 ,0x0800 ,56  ,127 ,40 ,100 ,0   },
{0x006  ,M_CGA2   ,640 ,200 ,80 ,25 ,8 ,8  ,4 ,0xB8000 ,0x0800 ,56  ,127 ,40 ,100 ,0   },
{0x008  ,M_TANDY16,160 ,200 ,20 ,25 ,8 ,8  ,8 ,0xB8000 ,0x2000 ,56  ,127 ,40 ,100 ,0   },
{0x009  ,M_TANDY16,320 ,200 ,40 ,25 ,8 ,8  ,8 ,0xB8000 ,0x2000 ,113 ,63  ,80 ,50  ,0   },
{0x00A  ,M_CGA4   ,640 ,200 ,80 ,25 ,8 ,8  ,8 ,0xB8000 ,0x2000 ,113 ,63  ,80 ,50  ,0   },
//{0x00E  ,M_TANDY16,640 ,200 ,80 ,25 ,8 ,8  ,8 ,0xA0000 ,0x10000 ,113 ,256 ,80 ,200 ,0   },
{0xFFFF  ,M_ERROR  ,0   ,0   ,0  ,0  ,0 ,0  ,0 ,0x00000 ,0x0000 ,0   ,0   ,0  ,0   ,0 },
};

VideoModeBlock Hercules_Mode=
{0x007  ,M_TEXT   ,640 ,400 ,80 ,25 ,8 ,14 ,1 ,0xB0000 ,0x1000 ,97 ,25  ,80 ,25  ,0};
*/
