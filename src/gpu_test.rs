// mentally close to cpu.rs, this is a collection of graphic tests using classic 256 / 512 byte demos

// TODO: copy all demo binaries that tests rely on to this repo

use cpu::CPU;
//use instruction::seg_offs_as_flat;
//use register::{AX, CS};
use tools;

/*
#[test]
fn demo_256_165plasm() {
    // STATUS: demo corrupts program code
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/165plasm/debug/165plasd.com");
    cpu.load_com(&code);

    // debug: run until ip = 0133
    let cs = cpu.sreg16[CS];
    cpu.add_breakpoint(seg_offs_as_flat(cs, 0x0133));
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
    // STATUS: pixels are rendered, but the effect is not proper, compare screenshots with real run (25 oct, 2017)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/244b/244b.com");
    cpu.load_com(&code);

    cpu.execute_n_instructions(1000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_244b_1k.png");
    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(49000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_244b_50k.png");
}

#[test]
fn demo_256_beziesux() {
    // STATUS: pixels are rendered, but the effect is not proper, compare screenshots with real run (26 oct, 2017)
    let mut cpu = CPU::new();
    let code = tools::read_binary("../dos-software-decoding/demo-256/beziesux/beziesux.com");
    cpu.load_com(&code);

    // XXX cpu.test_expect_memory_md5(x)

    cpu.execute_n_instructions(500_000);
    cpu.gpu.test_render_frame(&cpu.memory.memory, "tests/render/demo/256_beziesux_500k.png");
}
