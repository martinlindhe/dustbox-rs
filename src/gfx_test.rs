// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use cpu::CPU;
use instruction::seg_offs_as_flat;
use register::{AX, CS};
use tools;

/*
#[test]
fn demo_256_165plasm() {
    // STATUS: demo corrupts program code
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/165plasm/165plasm.com");
    cpu.load_com(&code);

    // debug: run until ip = 0131
    let cs = cpu.sreg16[CS];
    cpu.add_breakpoint(seg_offs_as_flat(cs, 0x0131));
    cpu.execute_n_instructions(1000);
    println!("{}", ".");
    println!("{}", ".");
    println!("{}", ".");
    cpu.clear_breakpoints();

    cpu.execute_n_instructions(400); // XXX hits corrupted op: "unknown op C8 at 085F:0164 (008754 flat), 1318 instructions executed"

// XXX write gfx frame as png
    //assert_eq!(0xFFFD, cpu.r16[AX].val);
}
*/

#[test]
fn demo_256_244b() {
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/244b/244b.com");
    cpu.load_com(&code);
    cpu.execute_n_instructions(10000);

// XXX write gfx frame as png
    //assert_eq!(0xFFFD, cpu.r16[AX].val);
    cpu.gpu.draw_canvas(); // XXX need such function that creates a raw image, not using cairo
}