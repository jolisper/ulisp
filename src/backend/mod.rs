pub mod llvm;
pub mod x86;

use crate::parser::Expression;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub(crate) struct BackendOptError {
    details: String,
}

impl BackendOptError {
    fn new(details: String) -> Self {
        BackendOptError { details }
    }
}

impl fmt::Display for BackendOptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for BackendOptError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub(crate) enum BackendOpt {
    LLVM,
    X86,
}

impl FromStr for BackendOpt {
    type Err = BackendOptError;
    fn from_str(backend: &str) -> Result<Self, Self::Err> {
        match backend {
            "x86" => Ok(BackendOpt::X86),
            "llvm" => Ok(BackendOpt::LLVM),
            _ => Err(BackendOptError::new(format!(
                "Unsupported backend: {}",
                backend
            ))),
        }
    }
}

pub(crate) trait Backend {
    type S;

    fn compile(&mut self, ast: &Expression) -> String;

    fn build(&mut self, asm: String, input: &str, output: &str);

    fn compile_expression(
        &mut self,
        arg: &Expression,
        destination: Option<&str>,
        scope: &mut Self::S,
    );

    fn compile_call(
        &mut self,
        function: &str,
        args: &[Expression],
        destination: Option<&str>,
        scope: &mut Self::S,
    );

    fn compile_define(
        &mut self,
        args: &[Expression],
        _destination: Option<&str>,
        scope: &mut Self::S,
    );

    fn compile_module(
        &mut self,
        args: &[Expression],
        destination: Option<&str>,
        scope: &mut Self::S,
    );

    fn emit<T>(&mut self, depth: usize, code: T)
    where
        T: Into<String>,
        Self: Sized;
}
