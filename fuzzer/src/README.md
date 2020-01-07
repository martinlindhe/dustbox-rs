# dustbox-fuzzer

-WIP-

Generates and encodes instruction sequences, and then runs them in
dustbox and a second target, comparing resulting registers and flags.

Used to verify instruction implementation correctness.

Currently the following code runners exists:

supersafe:

- Connects to an instance of the [supersafe](https://github.com/martinlindhe/supersafe) program running inside a VM.

vmrun:

- Uses the `vmrun` command line interface to execute programs inside a VMware Virtual Machine.
- Requires a password to be set in the guest VM in order to function.

dosbox-x:

- Uses the `dosbox-x` command line to execute programs inside a Dosbox-X environment.

## TODO

- take prober.com.tpl exact path as arg
- dosbox-x: verify that DosboxX runner works vs original dosbox project

- mutate 1, 2 and 3 operand forms of instrs

- LATER: bochs runner
- LATER: qemu runner

- com: implement superdos - a DOS program that uses the COM serial interface,
    and recieves binary data, executes it and sends back STDOUT over the wire,
    including checksums and re-transmit for real hardware and to be run inside
    dos emulator to speed things up.
    make use of https://crates.io/crates/serialport
    https://en.wikibooks.org/wiki/Serial_Programming/DOS_Programming
    https://www.dosbox.com/wiki/Configuration:SerialPort

    - use winXP + djgpp to build dos .exe ?
    - "use unix to build DOS programs" also exists at http://www.delorie.com/djgpp/zip-picker.html

    - supersafe.exe dont run in win98. linked to missing export KERNEL32.DLL:AddVectoredExceptionHandler
    - could run serial DOS program in win98 bare bones / vm
