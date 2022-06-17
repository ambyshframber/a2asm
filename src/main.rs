mod utils;
mod assembler;
mod avcmacro;

use assembler::Assembler;
use std::env::args;
use std::process::exit;
use std::fs::{read_to_string, write};

fn main() {
    exit(match run() {
        Ok(_) => 0,
        Err(e) => e
    })
}

fn run() -> Result<(), i32> {
    let args: Vec<String> = args().collect();
    let (in_file, out_file) = match args.len() {
        0 | 1 => return Err(2),
        2 => (args[1].as_str(), "out.avcr"),
        _ => (args[1].as_str(), args[2].as_str()),
    };
    let code = read_to_string(in_file).map_err(|_| 1)?;
    let mut asm = Assembler::new(&code);
    let rom = asm.assemble().unwrap();
    println!("assembly finished!");
    write(out_file, rom).map_err(|_| 1)?;

    Ok(())
}
