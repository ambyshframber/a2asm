# a2asm

a2asm is an assembler for the [AVC2 system](https://github.com/ambyshframber/avc2). It currently supports all instructions and several directives.

## Invoking a2asm

The basic usage is `a2asm INFILE [OUTFILE]`, where INFILE is the input assembly file, and OUTFILE is the desired output rom filename. If OUTFILE is not given, `out.avcr` is used.

## Instructions and directives

Instructions are notated using the syntax defined in the specification. A copy of the opcode table is included in this repository.

The current supported directives are:

- `.label(name)`: adds a label with the name given. `.lbl(name)` may also be used.
- `.absc(name)`: inserts the absolute address of the label given, as a raw value.
- `.x(hex)`: inserts the byte given as a raw value. `.hex(hex)` may also be used, but is deprecated.
- `.b(binary)`: inserts the byte given as a raw value.
- `.s(string)`: inserts the string given as a raw value.
- `.abspad(pad)`: pads to the specified location, given in hex. This can go backwards.
- `.defmac(name, args, content)`: defines a macro for later use.

Additionally, some shorthand directives are supported. `#hex` acts like `.x(hex)`. `"string` acts like `.s(string)`, with the caveat that spaces cannot be used. `'c` inserts the byte value of an ascii character. `%MACRO` or `%MACRO(args)` calls and expands a macro.

Comments are inserted using `//` or `/* ... */`. They function like in C, Rust, etc.

There is an implicit `.abspad(0300)` at the start of the program. Instructions and byte values cannot be added before 0x0300, but labels can. This can be used for mapping out the zero page.

Example programs can be found in the `examples` directory.

## Macros

The `.defmac` directive takes multiple arguments, some of which are also multiple values. An example declaration would be

```
.defmac(QUIT, (code), (LIT #$code LIT2 #ff #0f STA))
```

which could be invoked with `%QUIT(1)`. A more complicated example is

```
.defmac(ASMWORD, (name, lenflags, label, prev), (
    .lbl(name_$label)
    .abscall(name_$prev)
    .x($lenflags) .s(name)
    .lbl($label)
    .abscall(code_$label)
    .lbl(code_$label)
    POP2
))
```

which defines an asm primitive in avcforth.

`args` is a comma-separated list of arguments, which can later be referenced in the content. `content` is the actual code. Arguments can be referenced with `$ARG`, and a simple find-replace search is performed. It's not even close to Rust's `proc_macro` but it's better than Uxn. If a macro has no arguments, brackets do not need to be used.
