use crate::backend::Backend;
use crate::parser::Expression;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::Command;

pub type Scope = HashMap<String, String>;

type PrimitiveFunction =
    fn(&mut X86, args: &[Expression], destination: Option<&str>, scope: &mut Scope) -> ();

const PARAM_REGISTERS: &[&str] = &["rdi", "rsi", "rdx"];
const LOCAL_REGISTERS: &[&str] = &["rbx", "rbp", "r12"];

struct X86 {
    primitive_functions: HashMap<String, PrimitiveFunction>,
    builtin_functions: Scope,
    output: RefCell<String>,
}

impl X86 {
    fn new() -> Self {
        let primitive_functions = {
            let mut m = HashMap::<String, PrimitiveFunction>::new();
            m.insert("def".to_string(), X86::compile_define);
            m.insert("module".to_string(), X86::compile_module);
            m
        };
        let builtin_functions = {
            let mut m = HashMap::<String, String>::new();
            m.insert("+".to_string(), "plus".to_string());
            m
        };
        let output = RefCell::new(String::new());

        X86 {
            primitive_functions,
            builtin_functions,
            output,
        }
    }

    fn emit_prefix(&mut self) {
        self.emit(0, "; Generated with ulisp");
        self.emit(0, ";");
        self.emit(0, "; To compile run the following:");
        self.emit(0, "; $ nasm -f elf64 program.asm");
        self.emit(0, "; $ gcc -o program program.o");
        self.emit(0, "");

        self.emit(1, "global main\n");

        self.emit(1, "SECTION .text\n");

        self.emit(0, "plus:");
        self.emit(1, "add rdi, rsi");
        self.emit(1, "mov rax, rdi");
        self.emit(1, "ret\n");
    }

    fn emit_postfix(&mut self) {
        let mut syscall_map = HashMap::new();
        if cfg!(darwin) {
            syscall_map.insert("exit", "0x2000001");
        } else {
            syscall_map.insert("exit", "60");
        }

        self.emit(0, "main:");
        self.emit(1, "call program_main");
        self.emit(1, "mov rdi, rax");
        self.emit(1, format!("mov rax, {}", syscall_map["exit"]));
        self.emit(1, "syscall");
    }

    fn run_assembler(&mut self, asmfile: &str, codefile: &str) -> String {
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

    fn run_linker(&mut self, objfile: &str, binary: &str) {
        Command::new("gcc")
            .arg("-o")
            .arg(binary)
            .arg(objfile)
            .output()
            .expect("failed to run gcc");
    }

    fn write_asm(&mut self, output: &str, asm: String) {
        let mut output = fs::File::create(output).expect("failed open output file");
        output
            .write_all(asm.as_bytes())
            .expect("failed write output file");
    }
}

impl Backend for X86 {
    type S = Scope;

    fn compile(&mut self, ast: &Expression) -> String {
        self.emit_prefix();
        let mut scope = HashMap::<String, String>::new();
        self.compile_expression(ast, None, &mut scope);
        self.emit_postfix();

        self.output.borrow().to_string()
    }

    fn build(&mut self, asm: String, input: &str, output: &str) {
        let asmfile = &format!("{}.asm", input);
        self.write_asm(asmfile, asm);

        let objfile = self.run_assembler(asmfile, &input);
        self.run_linker(&objfile, &output);
    }

    fn compile_expression(
        &mut self,
        arg: &Expression,
        destination: Option<&str>,
        scope: &mut Scope,
    ) {
        #[allow(unused_assignments)]
        let mut origin: Option<String> = None;
        match arg {
            Expression::List(_vec) => {
                let (function, args) = split_function(arg);
                self.compile_call(&function, args, destination, scope);
                return;
            }
            Expression::Symbol(symbol) => {
                origin = if let Some(name) = scope.get(symbol) {
                    Some(name.to_string())
                } else {
                    panic!(
                        "Attempt to reference undefined variable or unsupported literal: {} ",
                        symbol
                    );
                };
            }
            Expression::Integer(int) => {
                origin = Some(format!("{}", int));
            }
            Expression::Float(_float) => {
                unimplemented!();
            }
            Expression::Boolean(_boolean) => {
                unimplemented!();
            }
        }
        self.emit(
            1,
            format!("mov {}, {}", destination.unwrap(), origin.unwrap()),
        );
    }

    fn compile_call(
        &mut self,
        function: &str,
        args: &[Expression],
        destination: Option<&str>,
        scope: &mut Scope,
    ) {
        if let Some(fun) = self.primitive_functions.get(function) {
            fun(self, args, destination, scope);
            return;
        }

        // Save param registers to the stack
        for (i, _) in args.iter().enumerate() {
            self.emit(1, format!("push {}", PARAM_REGISTERS[i]));
        }

        // Compile arguments and store in param registers
        for (i, arg) in args.iter().enumerate() {
            self.compile_expression(arg, Some(PARAM_REGISTERS[i]), scope);
        }

        // Call function
        self.emit(
            1,
            format!(
                "call {}",
                self.builtin_functions
                    .get(function)
                    .unwrap_or(&function.to_string())
            ),
        );

        for (i, _) in args.iter().enumerate() {
            self.emit(1, format!("pop {}", PARAM_REGISTERS[args.len() - i - 1]));
        }

        if let Some(d) = destination {
            self.emit(1, format!("mov {}, rax", d));
        }
    }

    fn compile_define(
        &mut self,
        args: &[Expression],
        _destination: Option<&str>,
        scope: &mut HashMap<String, String>,
    ) {
        let (name, params, body) = split_def_expression(args);

        self.emit(0, format!("{}:", name));

        let mut child_scope = scope.clone();
        for (i, param) in params.iter().enumerate() {
            if let Expression::Symbol(name) = param {
                let register = PARAM_REGISTERS[i].to_string();
                let local = LOCAL_REGISTERS[i].to_string();
                self.emit(1, format!("push {}", local));
                self.emit(1, format!("mov {}, {}", local, register));

                // Store parameter mapped to associated local
                child_scope.insert(name.to_string(), register);
            } else {
                panic!("Function param must be a symbol");
            };
        }

        self.compile_expression(body, Some("rax"), &mut child_scope);

        for (i, _) in params.iter().enumerate() {
            let local = LOCAL_REGISTERS[params.len() - i - 1].to_string();
            self.emit(1, format!("pop {}", local));
        }

        self.emit(1, "ret\n");
    }

    fn compile_module(
        &mut self,
        args: &[Expression],
        destination: Option<&str>,
        scope: &mut HashMap<String, String>,
    ) {
        for expression in args {
            self.compile_expression(expression, Some("rax"), scope);
        }
        if let Some(dest) = destination {
            if dest == "rax" {
            } else {
                self.emit(1, format!("mov {}, rax", dest));
            }
        }
    }

    fn emit<T>(&mut self, depth: usize, code: T)
    where
        T: Into<String>,
    {
        let mut indent = String::with_capacity(depth);
        for _ in 0..depth {
            indent.push_str("\t");
        }

        self.output
            .borrow_mut()
            .push_str(&format!("{}{}\n", indent, code.into()));
    }
}

pub(crate) fn new() -> Box<Backend<S = Scope>> {
    Box::new(X86::new())
}

fn split_function(list: &Expression) -> (String, &[Expression]) {
    if let Expression::List(vec) = list {
        if let Expression::Symbol(name) = &vec[0] {
            return (name.to_owned(), vec.split_at(1).1);
        } else {
            panic!("First list item is not a symbol");
        }
    } else {
        panic!("Expression is not a list item");
    }
}

fn split_def_expression(args: &[Expression]) -> (String, &Vec<Expression>, &Expression) {
    (
        if let Expression::Symbol(name) = &args[0] {
            let mut name = name.replace("-", "_");
            if name == "main" {
                name = "program_main".to_string();
            }
            name
        } else {
            panic!("First item must be a symbol in def statement");
        },
        if let Expression::List(vec) = &args[1] {
            vec
        } else {
            panic!("Second item must be a list in def statement");
        },
        if let Expression::List(_) = &args[2] {
            &args[2]
        } else {
            panic!("Third item must be a list in def statement");
        },
    )
}
