extern crate x86emu;

use x86emu::disasm;

#[test]
fn can_disassemble_basic_instructions() {
    let mut disasm = disasm::Disassembly::new();
    let code: Vec<u8> = vec![0xBA, 0x0B ,0x01, 0xB4, 0x09, 0xCD, 0x21];
    let res = disasm.disassemble(&code, 0x100);

    assert_eq!("0100: mov dx, 010B
0103: mov ah, 09
0105: int 21", res);
}
