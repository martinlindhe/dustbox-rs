// this is a collection of graphic tests using classic ms-dos demos

use std::panic;

use image::{ImageBuffer, Rgb, Pixel, GenericImage};

use crate::cpu::R;
use crate::machine::Machine;

#[test]
fn can_get_palette_entry() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB3, 0x03,         // mov bl,0x3
        0xB8, 0x15, 0x10,   // mov ax,0x1015
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x00, machine.cpu.get_r8(R::DH)); // red
    assert_eq!(0x2A, machine.cpu.get_r8(R::CH)); // green
    assert_eq!(0x2A, machine.cpu.get_r8(R::CL)); // blue
}

#[test]
fn can_set_palette_entry() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xBB, 0x03, 0x00,   // mov bx,0x3
        0xB5, 0x3F,         // mov ch,0x3f      ; red
        0xB1, 0x3F,         // mov cl,0x3f      ; green
        0xB6, 0x3F,         // mov dh,0x3f      ; blue
        0xB8, 0x10, 0x10,   // mov ax,0x1010
        0xCD, 0x10,         // int 0x10

        0xB3, 0x03,         // mov bl,0x3
        0xB8, 0x15, 0x10,   // mov ax,0x1015
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x3F, machine.cpu.get_r8(R::DH)); // red
    assert_eq!(0x3F, machine.cpu.get_r8(R::CH)); // green
    assert_eq!(0x3F, machine.cpu.get_r8(R::CL)); // blue
}

#[test]
fn can_get_font_info() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x30, 0x11,   // mov ax,0x1130  ; 1130 = get font info
        0xB7, 0x06,         // mov bh,0x6     ; get ROM 8x16 font (MCGA, VGA)
        0xCD, 0x10,         // int 0x10       ; es:bp = c000:1700 i dosbox
    ];
    machine.load_executable(&code);

    machine.execute_instructions(3);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0xC000, machine.cpu.get_r16(R::ES));
    assert_eq!(0x1700, machine.cpu.get_r16(R::BP));
}

#[test]
fn can_int10_put_pixel() {
    let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,   // mov ax,0x13
        0xCD, 0x10,         // int 0x10
        0xB4, 0x0C,         // mov ah,0xc       ; int 10h, ah = 0Ch
        0xB7, 0x00,         // mov bh,0x0
        0xB0, 0x0D,         // mov al,0xd       color
        0xB9, 0x01, 0x00,   // mov cx,0x1       x
        0xBA, 0x04, 0x00,   // mov dx,0x4       y
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(2);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x0113, machine.cpu.regs.ip);

    let frame = machine.gpu().unwrap().render_frame(&machine.mmu);
    let mut img = frame.draw_image();
    let img = img.sub_image(0, 0, 6, 6).to_image();
    assert_eq!("\
......
......
......
......
.O....
......
", draw_ascii(&img));
}

#[test]
fn can_write_vga_text() {
let mut machine = Machine::deterministic();
    let code: Vec<u8> = vec![
        0xB8, 0x13, 0x00,   // mov ax,0x13
        0xCD, 0x10,         // int 0x10
        0xB4, 0x0A,         // mov ah,0xa       ; int 10h, ah = 0Ah
        0xB0, 0x53,         // mov al,'S'       ; char
        0xB7, 0x00,         // mov bh,0x0       ; page
        0xB3, 0x01,         // mov bl,0x1       ; attrib
        0xB9, 0x01, 0x00,   // mov cx,0x1       ; count
        0xCD, 0x10,         // int 0x10
    ];
    machine.load_executable(&code);

    machine.execute_instructions(2);
    machine.execute_instruction(); // trigger the interrupt
    machine.execute_instructions(6);
    machine.execute_instruction(); // trigger the interrupt
    assert_eq!(0x0112, machine.cpu.regs.ip);

    let frame = machine.gpu().unwrap().render_frame(&machine.mmu);
    let mut img = frame.draw_image();
    let img = img.sub_image(0, 0, 8, 8).to_image();
    assert_eq!("\
.,,,,...
,,..,,..
,,,.....
.,,,....
...,,,..
,,..,,..
.,,,,...
........
", draw_ascii(&img));
}

fn draw_ascii(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> String {
    let mut res = String::new();
    for y in 0..img.height() {
        for x in 0..img.width() {
            let pixel = img.get_pixel(x, y);
            res.push(pixel_256_to_ascii(pixel));
        }
        res.push('\n');
    }
    res
}

fn pixel_256_to_ascii(v: &Rgb<u8>) -> char {
    let vals: [char; 9] = ['.', ',', '+', 'o', '5', '6', 'O', '0', '#'];
	let Rgb([r, g, b]) = v.to_rgb();
    let avg = (f64::from(r) + f64::from(g) + f64::from(b)) / 3.;
    let n = scale(avg, 0., 255., 0., (vals.len() - 1) as f64) as usize;
    assert_eq!(true, n <= vals.len());

    vals[n]
}

fn scale(value_in:f64, base_min:f64, base_max:f64, limit_min:f64, limit_max:f64) -> f64 {
	((limit_max - limit_min) * (value_in - base_min) / (base_max - base_min)) + limit_min
}
