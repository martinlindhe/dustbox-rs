#[macro_use]
extern crate criterion;

extern crate dustbox;

use criterion::Criterion;

use dustbox::cpu::CPU;
use dustbox::memory::mmu::MMU;

fn exec_simple_loop(c: &mut Criterion) {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0xB9, 0xFF, 0xFF, // mov cx,0xffff
        0x49,             // dec cx
        0xEB, 0xFA,       // jmp short 0x100
    ];

    cpu.load_com(&code);

    c.bench_function("execute small jmp short loop", |b| b.iter(|| cpu.execute_instruction()));
}

fn disasm_small_prog(c: &mut Criterion) {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let code: Vec<u8> = vec![
        0x80, 0x3E, 0x31, 0x10, 0x00,   // cmp byte [0x1031],0x0
        0xB9, 0xFF, 0xFF,               // mov cx,0xffff
        0x49,                           // dec cx
        0xEB, 0xFA,                     // jmp short 0x100
        0x83, 0xC7, 0x3A,               // add di,byte +0x3a
        0xBB, 0x8F, 0x79,               // mov bx,0x798f
        0xEB, 0xFA,                     // jmp short 0x100
        0xB9, 0xFF, 0xFF,               // mov cx,0xffff
    ];
    cpu.load_com(&code);

    c.bench_function("disasm small prog", |b| b.iter(|| cpu.decoder.disassemble_block_to_str(0x85F, 0x100, 8)));
}

criterion_group!(benches, exec_simple_loop, disasm_small_prog);
criterion_main!(benches);
