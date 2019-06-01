extern crate structopt;

mod backend;
mod parser;

use backend::llvm::Scope as llvm_Scope;
use backend::x86::Scope as x86_Scope;
use backend::{llvm, x86, Backend, BackendOpt};
use parser::{parse, Expression};
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
    #[structopt(short = "b", long = "backend", default_value = "llvm")]
    backend: BackendOpt,
}

type X86 = Box<dyn Backend<S = x86_Scope>>;
type LLVM = Box<dyn Backend<S = llvm_Scope>>;

fn main() {
    let opt = Opt::from_args();

    let input = opt.input.to_str().unwrap();
    let output = opt.output.to_str().unwrap();
    let backend = opt.backend;

    let code = read_input(input);
    let ast = parse(&code);

    match backend {
        BackendOpt::X86 => run_x86_backend(x86::new(), ast, input, output),
        BackendOpt::LLVM => run_llvm_backend(llvm::new(), ast, input, output),
    }
}

fn run_x86_backend(mut backend: X86, ast: Expression, input: &str, output: &str) {
    let asm = backend.compile(&ast);
    backend.build(asm, input, output);
}

fn run_llvm_backend(mut backend: LLVM, ast: Expression, input: &str, output: &str) {
    let asm = backend.compile(&ast);
    backend.build(asm, &input, &output);
}

fn read_input(input: &str) -> String {
    let mut input = fs::File::open(input).expect("failed open input file");
    let mut code = String::new();
    input
        .read_to_string(&mut code)
        .expect("failed read input file");
    code
}
