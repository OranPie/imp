use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub annos: Vec<String>,
    pub target: String,
    pub args: Vec<Arg>,
    pub line: usize,
}

impl Call {
    pub fn arg(&self, key: &str) -> Option<&Atom> {
        self.args
            .iter()
            .find(|arg| arg.key == key)
            .map(|arg| &arg.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub key: String,
    pub value: Atom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Ref(RefPath),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RefPath {
    pub namespace: String,
    pub name: String,
}

impl RefPath {
    pub fn parse(raw: &str) -> Option<Self> {
        let (namespace, name) = raw.split_once("::")?;
        if namespace.is_empty() || name.is_empty() {
            return None;
        }
        Some(Self {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse_program(src: &str) -> Result<Program, ParseError> {
    let mut calls = Vec::new();
    for (line, stmt) in split_statements(src)? {
        if stmt.trim().is_empty() {
            continue;
        }
        calls.push(parse_statement(&stmt, line)?);
    }
    Ok(Program { calls })
}

fn split_statements(src: &str) -> Result<Vec<(usize, String)>, ParseError> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut line = 1usize;
    let mut stmt_line = 1usize;

    for ch in src.chars() {
        if ch == '\n' {
            line += 1;
        }
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            current.push(ch);
            continue;
        }

        if ch == ';' {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                out.push((stmt_line, trimmed.to_owned()));
            }
            current.clear();
            stmt_line = line;
            continue;
        }

        if current.is_empty() && ch == '\n' {
            stmt_line = line;
        }
        current.push(ch);
    }

    if in_string {
        return Err(ParseError {
            line: stmt_line,
            message: "unterminated string literal".to_owned(),
        });
    }

    if !current.trim().is_empty() {
        return Err(ParseError {
            line: stmt_line,
            message: "statement must end with ';'".to_owned(),
        });
    }

    Ok(out)
}

fn parse_statement(stmt: &str, line: usize) -> Result<Call, ParseError> {
    let tokens = tokenize(stmt, line)?;
    if tokens.is_empty() {
        return Err(ParseError {
            line,
            message: "empty statement".to_owned(),
        });
    }

    if tokens[0] != "#call" {
        return Err(ParseError {
            line,
            message: "statement must start with #call".to_owned(),
        });
    }

    let mut index = 1;
    let mut annos = Vec::new();
    while index < tokens.len() && tokens[index].starts_with('@') {
        annos.push(tokens[index][1..].to_owned());
        index += 1;
    }

    let target = tokens.get(index).ok_or_else(|| ParseError {
        line,
        message: "missing target".to_owned(),
    })?;
    index += 1;

    let mut args = Vec::new();
    while index < tokens.len() {
        let token = &tokens[index];
        index += 1;
        let (key, raw_value) = token.split_once('=').ok_or_else(|| ParseError {
            line,
            message: format!("invalid key=value argument: {token}"),
        })?;
        if key.is_empty() {
            return Err(ParseError {
                line,
                message: "argument key cannot be empty".to_owned(),
            });
        }

        args.push(Arg {
            key: key.to_owned(),
            value: parse_atom(raw_value),
        });
    }

    Ok(Call {
        annos,
        target: target.to_owned(),
        args,
        line,
    })
}

fn tokenize(stmt: &str, line: usize) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in stmt.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(ch);
        if ch == '"' {
            in_string = true;
        }
    }

    if in_string {
        return Err(ParseError {
            line,
            message: "unterminated string literal".to_owned(),
        });
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn parse_atom(raw: &str) -> Atom {
    if raw == "null" {
        return Atom::Null;
    }
    if raw == "true" {
        return Atom::Bool(true);
    }
    if raw == "false" {
        return Atom::Bool(false);
    }
    if let Ok(num) = raw.parse::<f64>() {
        return Atom::Num(num);
    }
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        return Atom::Str(unescape_string(&raw[1..raw.len() - 1]));
    }
    if let Some(path) = RefPath::parse(raw) {
        return Atom::Ref(path);
    }
    Atom::Str(raw.to_owned())
}

fn unescape_string(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    other => {
                        out.push('\\');
                        out.push(other);
                    }
                }
            } else {
                out.push('\\');
            }
            continue;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_call_with_annos_and_refs() {
        let src = "#call @safe core::add a=local::x b=arg::y out=return::z;";
        let program = parse_program(src).expect("parse");
        assert_eq!(program.calls.len(), 1);
        let call = &program.calls[0];
        assert_eq!(call.annos, vec!["safe"]);
        assert_eq!(call.target, "core::add");
        assert_eq!(call.args.len(), 3);
    }

    #[test]
    fn parse_string_with_spaces() {
        let src = "#call core::host::print slot=local::x msg=\"hello world\";";
        let program = parse_program(src).expect("parse");
        let value = program.calls[0].arg("msg").expect("msg");
        assert_eq!(value, &Atom::Str("hello world".to_owned()));
    }
}
