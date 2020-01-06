use crate::gpu::modes::{ega_mode_block, vga_mode_block};


#[test]
fn is_mode_scales_correct() {
    // TODO find proper scale factors for the rest of the gfx modes

    for mode in &vga_mode_block() {
        let w = (mode.swidth as f32) * mode.scale_x;
        let h = (mode.sheight as f32) * mode.scale_y;

        let ar = w / h;
        if ar <= 1.32 || ar >= 1.34 {
            println!("incorrect ar {} for vga mode {:02X}: {}x{}", ar, mode.mode, mode.swidth, mode.sheight);
        }
    }

    for mode in &ega_mode_block() {
        let w = (mode.swidth as f32) * mode.scale_x;
        let h = (mode.sheight as f32) * mode.scale_y;

        let ar = w / h;
        if ar <= 1.32 || ar >= 1.34 {
            println!("incorrect ar {} for ega mode {:02X}: {}x{}", ar, mode.mode, mode.swidth, mode.sheight);
        }
    }

}