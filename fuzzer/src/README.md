# dustbox-fuzzer

-WIP-

Generates and encodes instruction sequences, and then runs them in
dustbox and a second target, comparing resulting registers and flags.

Used to verify instruction implementation correctness.

Currently the following code runners exists:

VmHttp:

- Connects to an instance of the [supersafe](https://github.com/martinlindhe/supersafe) program running inside a VM.

VmxVmrun:

- uses the `vmrun` command line interface to execute programs inside a VMware Virtual Machine.

DosboxX:

- uses the `dosbox-x` command line to execute programs inside a Dosbox-X environment.

## TODO
- cli arg: number of mutations per OP
- vmhttp: ip as cli argument

- deterministic mode toggle cli (give seed as arg)
 - take prober.com.tpl exact path as arg
- vmrun: able to specify VM name to execute program in on cli.
- vmrun: should be able to extract full path to vm:s from "vmrun" cmd if even needed ?
- dosboxx: verity that DosboxX runner works vs original dosbox project
- able to chose runner from cli

- mutate 1, 2 and 3 operand forms of instrs

- LATER: bochs runner
- LATER: qemu runner

- com: implement a DOS program that uses the COM interface,
    and recieves binary data, executes it and sends back STDOUT over the wire,
    including checksums and re-transmit for real hardware and to be run inside
    dos emulator to speed things up.
