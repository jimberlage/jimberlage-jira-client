use serde::{Serialize, Serializer};

fn escape_text_field(s: &str) -> String {
    let mut escaped_chars: Vec<char> = vec![];

    for c in s.chars() {
        match c {
            '"' => {
                escaped_chars.push('\\');
            }
            '+' | '-' | '&' | '|' | '!' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '~' | '*'
            | '?' | '\\' | ':' => {
                escaped_chars.push('\\');
                escaped_chars.push('\\');
            }
            _ => (),
        }

        escaped_chars.push(c);
    }

    escaped_chars.iter().collect()
}

#[derive(Debug, Clone)]
pub enum JQLValue {
    String(String),
    /* Float, Int, Uint, approved(), etc. would go here */
}

impl JQLValue {
    fn serialize_internal(&self) -> String {
        match self {
            JQLValue::String(contents) => format!("\"{}\"", escape_text_field(contents)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum JQLClause {
    And(Vec<Box<JQLClause>>),
    In(String, Vec<JQLValue>),
    /* OR, =, CONTAINS, etc. would go here */
}

impl JQLClause {
    fn serialize_internal(&self) -> String {
        match self {
            JQLClause::And(clauses) => {
                let joined_clauses = clauses
                    .iter()
                    .map(|clause| clause.serialize_internal())
                    .collect::<Vec<String>>()
                    .join(" AND ");
                format!("({})", joined_clauses)
            }
            JQLClause::In(field, values) => {
                let joined_values = values
                    .iter()
                    .map(|value| value.serialize_internal())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{} IN ({})", field, joined_values)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JQLStatement {
    pub clause: JQLClause,
    /* Order by would go here */
}

impl JQLStatement {
    pub fn serialize_internal(&self) -> String {
        self.clause.serialize_internal()
    }
}

impl Serialize for JQLStatement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let contents = self.serialize_internal();

        serializer.serialize_str(&contents)
    }
}
