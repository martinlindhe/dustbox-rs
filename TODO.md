# XXX

1. just cpu emulation,
    run actual ms-dos in the emu to register interrupt handlers etc...

later:
    high-level emu of ms-dos maybe , so we dont need to boot it.
    or use freedos ?




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

