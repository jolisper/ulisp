#[derive(Clone, Debug)]
pub enum Expression {
    List(Vec<Expression>),
    // Atoms:
    Symbol(String),
    Integer(i32),
    Float(f32),
    #[allow(dead_code)]
    Boolean(bool),
}

pub fn parse(program: &str) -> Expression {
    let mut tokens = tokenize(program);
    read_from_tokens(&mut tokens)
}

// Convert a string of characters into a list of tokens
fn tokenize(string: &str) -> Vec<String> {
    string
        .trim()
        .replace('(', "( ")
        .replace(')', " )")
        .replace("\n", "")
        .split(' ')
        .filter(|s| *s != "") // Empty list "()" generates List([Symbol("")])
        .map(std::borrow::ToOwned::to_owned)
        .collect()
}

// Read an expression from a sequence of tokens
fn read_from_tokens(mut tokens: &mut Vec<String>) -> Expression {
    if tokens.is_empty() {
        panic!("Unexpected EOF");
    }
    let token = tokens.remove(0);
    if token == "(" {
        let mut ts: Vec<Expression> = Vec::new();
        while tokens[0] != ")" {
            ts.push(read_from_tokens(&mut tokens));
        }
        tokens.remove(0);
        Expression::List(ts)
    } else if token == ")" {
        panic!("Syntax error");
    } else {
        atom(token.to_owned())
    }
}

// Select the appropiated atom type for the expression
fn atom(token: String) -> Expression {
    if let Ok(i) = str::parse::<i32>(&token) {
        return Expression::Integer(i);
    }
    if let Ok(f) = str::parse::<f32>(&token) {
        return Expression::Float(f);
    }
    Expression::Symbol(token)
}
