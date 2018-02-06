use fuzzer::*;
use dustbox::memory::mmu::MMU;

#[test] #[ignore] // expensive test
fn fuzz_instruction() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    let encoder = Encoder::new();

    let mut tot_sec = 0.;

    let affected_registers = vec!("ax");

    //The CF flag contains the value of the last bit shifted out of the destination operand; it is undefined for SHL and SHR instructions where the count is greater than or equal to the size (in bits) of the destination operand. 
    //The OF flag is affected only for 1-bit shifts (see “Description” above); otherwise, it is undefined.
    //The SF, ZF, and PF flags are set according to the result. If the count is 0, the flags are not affected.
    //For a non-zero count, the AF flag is undefined.
    let affected_flag_mask = AffectedFlags{c:1, o:1, s:1, z:1, p:1, a:1}.mask();

    for i in 1..65535 as usize {
        let n1 = ((i + 1) & 0xFF) ^ 0xFF;
        let n2 = i & 0xFF;
        let ops = vec!(
            // clear flags
            Instruction::new1(Op::Push16, Parameter::Imm16(0)),
            Instruction::new(Op::Popf),
            // clear ax,bx,cx,dx
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::AX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::BX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::CX), Parameter::Imm16(0)),
            Instruction::new2(Op::Mov16, Parameter::Reg16(R16::DX), Parameter::Imm16(0)),
            // mutate parameters
            Instruction::new2(Op::Mov8, Parameter::Reg8(R8::AH), Parameter::Imm8(n1 as u8)),
            Instruction::new2(Op::Ror8, Parameter::Reg8(R8::AH), Parameter::Imm8(n2 as u8)),
        );
        // XXX verified with winXP: Shr8, Rol8
        // XXX differs from winXP: Shl8 (OF), Sar8 (wrong result some times)
        //          - Ror8 (CF)

        if let Ok(data) = encoder.encode_vec(&ops) {
            // execute the ops in dustbox
            cpu.load_com(&data);
        } else {
            panic!("invalid data sequence");
        }

        cpu.execute_instructions(ops.len());

        // run in vm, compare regs
        let prober_com = "/Users/m/dev/rs/dustbox-rs/utils/prober/prober.com"; // XXX expand relative path
        assemble_prober(&ops, prober_com);

        let now = Instant::now();
        //let output = stdout_from_vmx_vmrun(prober_com); // ~2.3 seconds per call
        let output = stdout_from_vm_http(prober_com); // ~0.05 seconds
        //let output = stdout_from_dosbox(prober_com); // ~2.3 seconds

        let elapsed = now.elapsed();
        let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
        tot_sec += sec;
        if i % 100 == 0 {
            println!("avg vm time after {} iterations: {:.*}s", i, 4, tot_sec / i as f64);
        }
        let dustbox_ah = cpu.get_r8(&R8::AH);

        let vm_regs = prober_reg_map(&output);
        if compare_regs(&cpu, &vm_regs, &affected_registers) {
            println!("it {} ah={:02x} {{{:02x}, {:02x}}}: regs differ       (vm time {}s)", i, dustbox_ah, n1, n2, sec);
        }

        let vm_flags = vm_regs["flag"];
        let vm_masked_flags = vm_flags & affected_flag_mask;
        let dustbox_flags = cpu.flags.u16();
        let dustbox_masked_flags = dustbox_flags & affected_flag_mask;
        if vm_masked_flags != dustbox_masked_flags {
            let xored = vm_masked_flags ^ dustbox_masked_flags;
            print!("it {} ah={:02x} {{{:02x}, {:02x}}}: flags differ: vm {:04x}, dustbox {:04x} = diff b{:016b}: ", i, dustbox_ah, n1, n2, vm_masked_flags, dustbox_masked_flags, xored);
            // XXX show differing flag names
            if xored & 0x0000_0001 != 0 {
                print!("C ");
            }
            if xored & 0x0000_0004 != 0 {
                print!("P ");
            }
            if xored & 0x0000_0010 != 0 {
                print!("A ");
            }
            if xored & 0x0000_0040 != 0 {
                print!("Z ");
            }
            if xored & 0x0000_0080 != 0 {
                print!("S ");
            }
            if xored & 0x0000_0800 != 0 {
                print!("O ");
            }
            println!();
        } else {
            print!(".");
            io::stdout().flush().ok().expect("Could not flush stdout");
        }

    }
}
