# a2asm

a2asm is an assembler for the [AVC2 system](https://github.com/ambyshframber/avc2). It currently supports all instructions and 3 directives (label, absc and hex).

## Invoking a2asm

The basic usage is `a2asm INFILE [OUTFILE]`, where INFILE is the input assembly file, and OUTFILE is the desired output rom filename. If OUTFILE is not given, `out.avcr` is used.

## Instructions and directives

Instructions are notated using the syntax defined in the specification. A copy of the opcode table is included in this repository.

The current supported directives are:

- `.label(name)`: adds a label with the name given. `.lbl(name)` may also be used.
- `.absc(name)`: inserts the absolute address of the label given, as a raw value.
- `.hex(hex)`: inserts the byte given as a raw value.

Comments are inserted using `\`. They proceed until either the end of the line or another `\`.

Example programs can be found in the `examples` directory.
