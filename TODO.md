# XXX


0. visual diff output test:
    run all dos-software-decoding samples 1 million instructions each,
    screengrab and save to file

    visual diff images between runs

pro: allows for quick testing of many samples


1. measure how many instructions is run in 1 second, to get a Millions of instructions per second (MIPS) value


1. just cpu emulation,
    run actual ms-dos in the emu to register interrupt handlers etc...

later:
    high-level emu of ms-dos maybe , so we dont need to boot it.
    or pack with freedos/2dos or similar ?




# NOW - REWORK:

have a single instruction decoder,
    which decodes arguments,
    and is then rendered by the disassembler,
    so for disasm we have:
        1. op lookup table -> instr name
        2. instr decode -> get arguments
        3. render
    
    thus disasm can be a tiny part of the cpu module,
    rather than a separate module.

