extern crate structopt;

mod backend;
mod compiler;
mod parser;

use backend::BackendOpt;
use parser::parse;
use std::fs;
use std::io::Read;
use std::path;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: path::PathBuf,
    #[structopt(short = "o", long = "output", default_value = "a.out")]
    output: path::PathBuf,
    #[structopt(short = "b", long = "backend", default_value = "x86")]
    backend: BackendOpt,
}

fn main() {
    let opt = Opt::from_args();

    let input = opt.input.to_str().unwrap();
    let output = opt.output.to_str().unwrap();
    let backend = opt.backend;

    let code = read_input(input);   
    let ast = parse(&code);

    let mut backend = backend::create(backend);
    let asm = backend.compile(&ast);
    backend.build(asm, &input, &output);
}

fn read_input(input: &str) -> String {
    let mut input = fs::File::open(input)
        .expect("failed open input file");
    let mut code = String::new();
    input.read_to_string(&mut code)
        .expect("failed read input file");
    code
}
