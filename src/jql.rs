use chrono::NaiveDate;
use serde::{Serialize, Serializer};

/// Escapes text for use in a JQL query.
///
/// See ["Restricted words and characters"][1] to see where these escape characters are sourced from.
///
/// ### Example
///
/// ```
/// use jimberlage_jira_client::jql;
///
/// assert_eq!(jql::escape_text_field("foo.bar@example.com"), "\"foo.bar@example.com\"".to_owned());
/// assert_eq!(jql::escape_text_field("[foo]:(bar)"), "\"\\\\[foo\\\\]\\\\:\\\\(bar\\\\)\"".to_owned());
/// ```
///
/// [1]: https://support.atlassian.com/jira-software-cloud/docs/what-is-advanced-searching-in-jira-cloud/#Advancedsearching-restrictionsRestrictedwordsandcharacters
pub fn escape_text_field(s: &str) -> String {
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

    format!("\"{}\"", escaped_chars.iter().collect::<String>())
}

/// Represents an object that has a string representation in JQL, either as a standalone query or as part of a query.
pub trait SerializableToJQL {
    fn serialize_to_jql(&self) -> String;
}

/// Represents a [value][1] in JQL.
///
/// Right now, this just represents values I demonstrably use in my own code.
/// Values can also be numeric, so it may make sense to extend this enum with numeric types.
///
/// [1]: https://support.atlassian.com/jira-software-cloud/docs/what-is-advanced-searching-in-jira-cloud/#Advancedsearching-ConstructingJQLqueries
#[derive(Debug, Clone)]
pub enum JQLValue {
    String(String),
    NaiveDate(NaiveDate),
    /* Float, Int, Uint, approved(), etc. would go here */
}

impl SerializableToJQL for JQLValue {
    /// Serialize the JQL value to its representation as part of a string.
    ///
    /// This involves escaping string fields appropriately.
    ///
    /// ### Example
    ///
    /// ```
    /// use jimberlage_jira_client::jql::{self, SerializableToJQL};
    ///
    /// assert_eq!(jql::JQLValue::String("Hello world".to_owned()).serialize_to_jql(), "\"Hello world\"".to_owned());
    /// assert_eq!(jql::JQLValue::String("^latest".to_owned()).serialize_to_jql(), "\"\\\\^latest\"".to_owned());
    /// ```
    fn serialize_to_jql(&self) -> String {
        match self {
            JQLValue::String(contents) => escape_text_field(contents),
            JQLValue::NaiveDate(date) => format!("\"{}\"", date.format("%Y-%m-%d").to_string()),
        }
    }
}

/// Represents a [clause][1] in JQL.
///
/// Right now, this just represents clauses I demonstrably use in my own code.
/// There are more clauses than these, so it may make sense to extend this enum.
///
/// [1]: https://support.atlassian.com/jira-software-cloud/docs/what-is-advanced-searching-in-jira-cloud/#Advancedsearching-ConstructingJQLqueries
#[derive(Debug, Clone)]
pub enum JQLClause {
    And(Vec<Box<JQLClause>>),
    Equals(String, JQLValue),
    GreaterThanEquals(String, JQLValue),
    In(String, Vec<JQLValue>),
    LessThanEquals(String, JQLValue),
    /* OR, ~, CONTAINS, etc. would go here */
}

impl SerializableToJQL for JQLClause {
    /// Serialize the JQL clause to its representation as part of a string.
    ///
    /// This involves formatting values correctly, and ensuring operator precedence rules are respected.
    ///
    /// ### Example
    ///
    /// ```
    /// use jimberlage_jira_client::jql::{self, JQLClause, JQLValue, SerializableToJQL};
    ///
    /// assert_eq!(
    ///     JQLClause::In("project".to_owned(), vec![]).serialize_to_jql(),
    ///     "project IN ()".to_owned()
    /// );
    /// assert_eq!(
    ///     JQLClause::In("project".to_owned(), vec![JQLValue::String("SRE".to_owned())]).serialize_to_jql(),
    ///     "project IN (\"SRE\")".to_owned()
    /// );
    /// assert_eq!(
    ///     JQLClause::In("project".to_owned(), vec![JQLValue::String("PE".to_owned()), JQLValue::String("SRE".to_owned())]).serialize_to_jql(),
    ///     "project IN (\"PE\", \"SRE\")".to_owned()
    /// );
    /// assert_eq!(JQLClause::And(vec![]).serialize_to_jql(), "()".to_owned());
    /// assert_eq!(
    ///     JQLClause::And(vec![
    ///         Box::new(JQLClause::In("project".to_owned(), vec![JQLValue::String("SRE".to_owned())]))
    ///     ]).serialize_to_jql(),
    ///     "(project IN (\"SRE\"))".to_owned()
    /// );
    /// assert_eq!(
    ///     JQLClause::And(vec![
    ///         Box::new(JQLClause::In("project".to_owned(), vec![JQLValue::String("SRE".to_owned())])),
    ///         Box::new(JQLClause::In("labels".to_owned(), vec![JQLValue::String("v2022.5.10".to_owned()), JQLValue::String("v2022.6.13".to_owned())]))
    ///     ]).serialize_to_jql(),
    ///     "(project IN (\"SRE\") AND labels IN (\"v2022.5.10\", \"v2022.6.13\"))".to_owned()
    /// );
    /// ```
    fn serialize_to_jql(&self) -> String {
        match self {
            JQLClause::And(clauses) => {
                let joined_clauses = clauses
                    .iter()
                    .map(|clause| clause.serialize_to_jql())
                    .collect::<Vec<String>>()
                    .join(" AND ");
                format!("({})", joined_clauses)
            }
            JQLClause::Equals(field, value) => {
                format!("{} = {}", field, value.serialize_to_jql())
            }
            JQLClause::GreaterThanEquals(field, value) => {
                format!("{} >= {}", field, value.serialize_to_jql())
            }
            JQLClause::In(field, values) => {
                let joined_values = values
                    .iter()
                    .map(|value| value.serialize_to_jql())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{} IN ({})", field, joined_values)
            }
            JQLClause::LessThanEquals(field, value) => {
                format!("{} <= {}", field, value.serialize_to_jql())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum JQLOrdering {
    Asc,
    Desc,
}

impl SerializableToJQL for JQLOrdering {
    fn serialize_to_jql(&self) -> String {
        match self {
            JQLOrdering::Asc => "ASC".to_owned(),
            JQLOrdering::Desc => "DESC".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JQLOrderByPart {
    pub field: String,
    pub ordering: Option<JQLOrdering>,
}

impl SerializableToJQL for JQLOrderByPart {
    fn serialize_to_jql(&self) -> String {
        match &self.ordering {
            Some(ordering) => format!("{} {}", self.field, ordering.serialize_to_jql()),
            None => self.field.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct JQLOrderBy(pub Vec<JQLOrderByPart>);

impl SerializableToJQL for JQLOrderBy {
    fn serialize_to_jql(&self) -> String {
        let serialized_fields = self
            .0
            .iter()
            .map(|order| order.serialize_to_jql())
            .collect::<Vec<String>>()
            .join(", ");

        format!("ORDER BY {}", serialized_fields)
    }
}

/// Represents a [statement][1] in JQL.
///
/// Right now, this just represents parts of a statement I demonstrably use in my own code.
/// It may make sense to add fields to this struct.
///
/// [1]: https://support.atlassian.com/jira-software-cloud/docs/what-is-advanced-searching-in-jira-cloud/#Advancedsearching-ConstructingJQLqueries
#[derive(Debug, Clone)]
pub struct JQLStatement {
    pub clause: JQLClause,
    pub order_by: Option<JQLOrderBy>,
}

impl SerializableToJQL for JQLStatement {
    /// Serialize the JQL statement to its representation as part of a string.
    ///
    /// This involves formatting values correctly, and ensuring operator precedence rules are respected.
    /// It may optionally involve setting an order by on the statement as well.
    fn serialize_to_jql(&self) -> String {
        match &self.order_by {
            Some(order_by) if order_by.0.is_empty() => self.clause.serialize_to_jql(),
            Some(order_by) => format!(
                "{} {}",
                self.clause.serialize_to_jql(),
                order_by.serialize_to_jql()
            ),
            None => self.clause.serialize_to_jql(),
        }
    }
}

impl Serialize for JQLStatement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let contents = self.serialize_to_jql();

        serializer.serialize_str(&contents)
    }
}
