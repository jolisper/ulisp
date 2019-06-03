#[cfg(test)]
mod tests;

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Scope {
    locals: HashMap<String, String>,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            locals: HashMap::new(),
        }
    }

    pub fn register(&mut self, local: String) -> String {
        let mut copy = safe_name(&local);
        let mut n = 1;
        while self.locals.get(&copy).is_some() {
            copy = format!("{}{}", local, n);
            n += 1;
        }
        self.locals.insert(local, copy.to_owned());
        copy
    }

    pub fn symbol(&mut self, prefix: Option<&str>) -> String {
        let nth = self.locals.len() + 1;
        let prefix = prefix.unwrap_or_else(|| "sym");
        self.register(format!("{}{}", prefix, nth))
    }

    pub fn get(&mut self, local: &str) -> Option<String> {
        match self.locals.get(local) {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub fn copy(&mut self) -> Scope {
        self.clone()
    }   
}

pub(crate) fn safe_name(symbol_name: &str) -> String {
    if symbol_name == "-" {
        return symbol_name.to_owned();
    }
    symbol_name.replace("-", "_")
}
