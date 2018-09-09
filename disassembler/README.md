# TODO

## tracer

- symbolic execution, eg understand that this snippet exits application so parsing should stop after the int:

 ../dos-software-decoding/demo-256/fire2/fire2.com:
    [085F:01AC] B44C             Mov8     ah, 0x4C
    [085F:01AE] CD21             Int      0x21          ; DOS 2+ - EXIT - TERMINATE WITH RETURN CODE

../dos-software-decoding/demo-256/optimize/optimize.com:
    [085F:01EA] CD20             Int      0x20          ; DOS 1+ - TERMINATE PROGRAM

- remember data offsets and sizes (bytes, words, strings... ?)
