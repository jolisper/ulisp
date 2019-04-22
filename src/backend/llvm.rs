use std::collections::HashMap;

struct Scope {
    locals: HashMap<String, String>,
}

impl Scope {
    fn new() -> Self {
        Scope {
            locals: HashMap::new(),
        }
    }

    fn register(&mut self, local: String) -> String {
        let mut copy = local.replace("-", "_");
        let mut n = 1;
        while self.locals.get(&copy).is_some() {
            n += 1;
            copy = format!("{}{}", local, n);
        }
        self.locals.insert(local, copy.to_owned());
        copy
    }

    fn symbol(&mut self) -> String {
        let nth = self.locals.len();
        self.register(format!("{}{}", "sym", nth))
    }

    fn get(&mut self, local: String) -> Option<&String> {
        self.locals.get(&local)
    }

    fn copy(&mut self) -> HashMap<String, String> {
        self.locals.clone()
    }
}