extern crate x86emu;

use x86emu::cpu;

#[test]
fn can_disassemble_basic_instructions() {
    let mut cpu = cpu::CPU::new();
    let code: Vec<u8> = vec![0xBA, 0x0B ,0x01, 0xB4, 0x09, 0xCD, 0x21];
    cpu.load_rom(&code, 0x100);


    // MOV DX, 0x10B
    cpu.execute_instruction();
    assert_eq!(0x103, cpu.pc);
    assert_eq!(0x10B, cpu.r16[cpu::DX]);
}
