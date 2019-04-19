use crate::parser::Expression;
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::collections::HashMap;

type PrimitiveFunction =
    fn(args: &[Expression], destination: Option<&str>, scope: &mut HashMap<String, String>) -> ();

lazy_static! {
    static ref PRIMITIVE_FUNCTIONS: HashMap<String, PrimitiveFunction> = {
        let mut m = HashMap::<String, PrimitiveFunction>::new();
        m.insert("def".to_string(), compile_define);
        m.insert("module".to_string(), compile_module);
        m
    };
    static ref BUILTIN_FUNCTIONS: HashMap<String, String> = {
        let mut m = HashMap::<String, String>::new();
        m.insert("+".to_string(), "plus".to_string());
        m
    };
}

thread_local! {
    static OUTPUT: RefCell<String> = RefCell::new(String::new());
}

const PARAM_REGISTERS: &[&str] = &["rdi", "rsi", "rdx"];
const LOCAL_REGISTERS: &[&str] = &["rbx", "rbp", "r12"];

pub fn compile(ast: &Expression) -> String {
    emit_prefix();
    let mut scope = HashMap::<String, String>::new();
    compile_expression(ast, None, &mut scope);
    emit_postfix();

    OUTPUT.with(|f| f.borrow().to_string())
}

fn compile_expression(
    arg: &Expression,
    destination: Option<&str>,
    scope: &mut HashMap<String, String>,
) {
    #[allow(unused_assignments)]
    let mut origin: Option<String> = None;
    match arg {
        Expression::List(_vec) => {
            let (function, args) = split_function(arg);
            compile_call(&function, args, destination, scope);
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
    emit(
        1,
        format!("mov {}, {}", destination.unwrap(), origin.unwrap()),
    );
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

fn compile_call(
    function: &str,
    args: &[Expression],
    destination: Option<&str>,
    scope: &mut HashMap<String, String>,
) {
    if let Some(fun) = PRIMITIVE_FUNCTIONS.get(function) {
        fun(args, destination, scope);
        return;
    }

    // Save param registers to the stack
    for (i, _) in args.iter().enumerate() {
        emit(1, format!("push {}", PARAM_REGISTERS[i]));
    }

    // Compile arguments and store in param registers
    for (i, arg) in args.iter().enumerate() {
        compile_expression(arg, Some(PARAM_REGISTERS[i]), scope);
    }

    // Call function
    emit(
        1,
        format!(
            "call {}",
            BUILTIN_FUNCTIONS
                .get(function)
                .unwrap_or(&function.to_string())
        ),
    );

    for (i, _) in args.iter().enumerate() {
        emit(1, format!("pop {}", PARAM_REGISTERS[args.len() - i - 1]));
    }

    if let Some(d) = destination {
        emit(1, format!("mov {}, rax", d));
    }
}

fn compile_define(
    args: &[Expression],
    _destination: Option<&str>,
    scope: &mut HashMap<String, String>,
) {
    let (name, params, body) = split_def_expression(args);

    emit(0, format!("{}:", name));

    let mut child_scope = scope.clone();
    for (i, param) in params.iter().enumerate() {
        if let Expression::Symbol(name) = param {
            let register = PARAM_REGISTERS[i].to_string();
            let local = LOCAL_REGISTERS[i].to_string();
            emit(1, format!("push {}", local));
            emit(1, format!("mov {}, {}", local, register));

            // Store parameter mapped to associated local
            child_scope.insert(name.to_string(), register);
        } else {
            panic!("Function param must be a symbol");
        };
    }

    compile_expression(body, Some("rax"), &mut child_scope);

    for (i, _) in params.iter().enumerate() {
        let local = LOCAL_REGISTERS[params.len() - i - 1].to_string();
        emit(1, format!("pop {}", local));
    }

    emit(1, "ret\n");
}

fn split_def_expression(args: &[Expression]) -> (String, &Vec<Expression>, &Expression) {
    (
        if let Expression::Symbol(name) = &args[0] {
            let mut name = name.replace("-", "_");
            if name == "main" {
                name =  "program_main".to_string();
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

fn compile_module(
    args: &[Expression],
    destination: Option<&str>,
    scope: &mut HashMap<String, String>,
) {
    for expression in args {
        compile_expression(expression, Some("rax"), scope);
    }
    if let Some(dest) = destination {
        if dest == "rax" {
        } else {
            emit(1, format!("mov {}, rax", dest));
        }
    }
}

fn emit<T>(depth: usize, code: T)
where
    T: Into<String>,
{
    let mut indent = String::with_capacity(depth);
    for _ in 0..depth {
        indent.push_str("\t");
    }
    OUTPUT.with(|f| {
        f.borrow_mut()
            .push_str(&format!("{}{}\n", indent, code.into()));
    })
}

fn emit_prefix() {
    emit(0, "; Generated with ulisp");
    emit(0, ";");
    emit(0, "; To compile run the following:");
    emit(0, "; $ nasm -f elf64 program.asm");
    emit(0, "; $ gcc -o program program.o");
    emit(0, "");

    emit(1, "global main\n");

    emit(1, "SECTION .text\n");

    emit(0, "plus:");
    emit(1, "add rdi, rsi");
    emit(1, "mov rax, rdi");
    emit(1, "ret\n");
}

fn emit_postfix() {
    let mut syscall_map = HashMap::new();
    if cfg!(darwin) {
        syscall_map.insert("exit", "0x2000001");
    } else {
        syscall_map.insert("exit", "60");
    }

    emit(0, "main:");
    emit(1, "call program_main");
    emit(1, "mov rdi, rax");
    emit(1, format!("mov rax, {}", syscall_map["exit"]));
    emit(1, "syscall");
}
