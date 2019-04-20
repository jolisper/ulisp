extern crate structopt;

mod compiler;
mod parser;

use compiler::compile;
use parser::parse;
use std::fs;
use std::io::{Read, Write};
use std::path;
use std::process::Command;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: path::PathBuf,
    #[structopt(short = "o", long = "output", default_value = "a.out")]
    output: path::PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let input = opt.input.to_str().unwrap();
    let output = opt.output.to_str().unwrap();

    let code = read_input(input);   
    let ast = parse(&code);
    let asm = compile(&ast);

    let asmfile = &format!("{}.asm", input);
    write_asm(asmfile, asm);

    let objfile = run_assembler(asmfile, input);
    
    run_linker(&objfile, &output);
}

fn read_input(input: &str) -> String {
    let mut input = fs::File::open(input)
        .expect("failed open input file");
    let mut code = String::new();
    input.read_to_string(&mut code)
        .expect("failed read input file");
    code
}

fn write_asm(output: &str, asm: String) {
    let mut output = fs::File::create(output)
        .expect("failed open output file");
    output.write_all(asm.as_bytes())
        .expect("failed write output file");
}

fn run_assembler(asmfile: &str, codefile: &str) -> String {
    let objfile = format!("{}.o", codefile);
    Command::new("nasm")
        .arg("-f")
        .arg("elf64")
        .arg("-o")
        .arg(&objfile)
        .arg(asmfile)
        .output()
        .expect("failed to run nasm");
    objfile
}

fn run_linker(objfile: &str, binary: &str) {
    Command::new("gcc")
        .arg("-o")
        .arg(binary)
        .arg(objfile)
        .output()
        .expect("failed to run gcc");
}