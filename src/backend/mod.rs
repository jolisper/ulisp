mod llvm;
mod x86;

use std::fmt;
use std::str::FromStr;
use crate::parser::Expression;

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
    fn compile(&mut self, ast: &Expression) -> String;

    fn build(&mut self, asm: String, input: &str, output: &str);
}

pub(crate) fn create(backend: BackendOpt) -> Box<Backend> {
    match backend {
        BackendOpt::LLVM => unimplemented!("llvm backend not implemented, yet"),
        BackendOpt::X86 => x86::new(),
    }
}