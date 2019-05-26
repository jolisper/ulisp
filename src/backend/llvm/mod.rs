use crate::backend::Backend;
use crate::parser::Expression;
use scope::safe_name;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::rc::Rc;

pub use scope::Scope;

mod scope;

type PrimitiveFunction = Rc<Fn(&mut LLVM, &[Expression], Option<&str>, &mut Scope) -> ()>;

struct LLVM {
    output: String,
    primitive_functions: HashMap<String, PrimitiveFunction>,
}

impl Backend for LLVM {
    type S = Scope;

    fn emit<T>(&mut self, depth: usize, code: T)
    where
        T: Into<String>,
    {
        let mut indent = String::with_capacity(depth);
        for _ in 0..depth {
            indent.push_str("\t");
        }
        let s: String = code.into();
        self.output.push_str(&format!("{}{}\n", indent, s.clone()));

        //println!("{}{}", indent, s.clone());
    }

    fn compile(&mut self, ast: &Expression) -> String {
        let mut scope = Scope::new();
        let destination = scope.symbol();
        self.compile_expression(ast, Some(&destination), &mut scope);
        self.output.clone()
    }

    fn compile_expression(
        &mut self,
        arg: &Expression,
        destination: Option<&str>,
        scope: &mut Scope,
    ) {
        match arg {
            Expression::List(_vec) => {
                let (function, args) = split_function(arg);
                //let dest = scope.symbol();
                self.compile_call(&function, args, destination, scope);
                //self.compile_call(&function, args, Some(&dest), scope);
                return;
            }
            Expression::Symbol(symbol) => {
                if let Some(name) = scope.get(symbol) {
                    self.emit(
                        1,
                        format!("%{} = add i32 %{}, 0", destination.unwrap(), name),
                    );
                } else {
                    panic!(
                        "Attempt to reference undefined variable or unsupported literal: {} ",
                        symbol
                    );
                };
            }
            Expression::Integer(int) => {
                self.emit(1, format!("%{} = add i32 {}, 0", destination.unwrap(), int));
            }
            Expression::Float(_float) => {
                unimplemented!();
            }
            Expression::Boolean(_boolean) => {
                unimplemented!();
            }
        }
    }

    fn compile_call(
        &mut self,
        function: &str,
        args: &[Expression],
        destination: Option<&str>,
        scope: &mut Scope,
    ) {
        if let Some(fun) = self.get_primitive_function(function) {
            let mut scope = scope;
            (*fun)(self, args, destination, &mut scope);
            return;
        }

        let valid_function = if let Some(f) = scope.get(function) {
            f
        } else {
            //println!("{:?} => {:?}", function, self.primitive_functions.get(function).is_some());
            panic!("Attempt to call undefined function: {}", function);
        };

        let safe_args = args
            .iter()
            .map(|arg| {
                let sym = scope.symbol();
                self.compile_expression(arg, Some(&sym), scope);
                format!("i32 %{}", sym)
            })
            .fold("".to_string(), |acc, s| {
                if acc == "" {
                    s.to_string()
                } else {
                    format!("{}, {}", acc, s)
                }
            });

        self.emit(
            1,
            format!(
                "%{} = call i32 @{}({})",
                destination.unwrap(),
                valid_function,
                safe_args
            ),
        );
    }

    fn compile_define(
        &mut self,
        args: &[Expression],
        _destination: Option<&str>,
        scope: &mut Scope,
    ) {
        let (name, params, body) = split_def_expression(args);
        // Add this function to outer scope
        let safe_name = scope.register(name);
        // Copy outer scope so parameter mappings aren't exposed in outer scope.
        let mut child_scope = scope.copy();

        let safe_params = params
            .iter()
            .map(|param| {
                if let Expression::Symbol(param_name) = param {
                    child_scope.register(param_name.to_string())
                } else {
                    panic!("")
                }
            })
            .fold("".to_string(), |acc, s| {
                if acc == "" {
                    format!("i32 %{}", s)
                } else {
                    format!("{}, i32 %{}", acc, s)
                }
            });

        self.emit(0, format!("define i32 @{}({}) {{", safe_name, safe_params));

        let ret = child_scope.symbol();
        //println!("ret={}", ret);
        self.compile_expression(body, Some(&ret), &mut child_scope);

        self.emit(1, format!("ret i32 %{}", ret));
        self.emit(0, "}\n");
    }

    fn compile_module(
        &mut self,
        args: &[Expression],
        _destination: Option<&str>,
        scope: &mut Scope,
    ) {
        for expression in args {
            self.compile_expression(expression, None, scope);
        }
    }

    fn build(&mut self, asm: String, input: &str, output: &str) {
        let asmfile = &format!("{}.ll", input);
        self.write_asm(asmfile, asm);

        let objfile = self.run_assembler(asmfile, &input);
        self.run_linker(&objfile, &output);
    }
}

impl LLVM {
    fn new() -> Self {
        let primitive_functions = {
            let mut m = HashMap::<String, PrimitiveFunction>::new();
            m.insert("def".to_string(), Rc::new(Self::compile_define));
            m.insert("module".to_string(), Rc::new(Self::compile_module));
            m.insert("+".to_string(), Self::compile_operation("add"));
            m.insert("-".to_string(), Self::compile_operation("sub"));
            m.insert("*".to_string(), Self::compile_operation("mul"));
            m
        };
        let output = String::new();

        LLVM {
            primitive_functions,
            output,
        }
    }

    fn get_primitive_function(&mut self, name: &str) -> Option<PrimitiveFunction> {
        match self.primitive_functions.get(name) {
            Some(pf) => Some(pf.clone()),
            None => None,
        }
    }

    fn compile_operation<T: 'static>(operation: T) -> PrimitiveFunction
    where
        T: Into<String> + Clone,
    {
        let c = move |backend: &mut LLVM,
                      expressions: &[Expression],
                      destination: Option<&str>,
                      scope: &mut Scope| {
            let exp1 = &expressions[0];
            let exp2 = &expressions[1];

            let arg1 = scope.symbol();
            let arg2 = scope.symbol();

            backend.compile_expression(exp1, Some(&arg1), scope);
            backend.compile_expression(exp2, Some(&arg2), scope);
            backend.emit(
                1,
                format!(
                    "%{} = {} i32 %{}, %{}",
                    destination.unwrap(),
                    operation.clone().into(),
                    arg1,
                    arg2
                ),
            );
        };
        Rc::new(c)
    }

    fn write_asm(&mut self, output: &str, asm: String) {
        let mut output = fs::File::create(output).expect("failed open output file");
        output
            .write_all(asm.as_bytes())
            .expect("failed write output file");
    }

    fn run_assembler(&mut self, asmfile: &str, codefile: &str) -> String {
        let objfile = format!("{}.s", codefile);
        Command::new("llc")
            .arg("-o")
            .arg(&objfile)
            .arg(asmfile)
            .output()
            .expect("failed to run llc");
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
}

fn split_function(list: &Expression) -> (String, &[Expression]) {
    if let Expression::List(vec) = list {
        if let Expression::Symbol(name) = &vec[0] {
            return (safe_name(name), vec.split_at(1).1);
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
            safe_name(name)
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

pub(crate) fn new() -> Box<Backend<S = Scope>> {
    Box::new(LLVM::new())
}
