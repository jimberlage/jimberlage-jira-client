use std::collections::HashMap;

use base64::{
    self,
    engine::{GeneralPurpose, GeneralPurposeConfig},
    Engine,
};
use reqwest::{
    self,
    blocking::{Client, ClientBuilder, RequestBuilder},
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::value::Value as JSONValue;

use self::jql::JQLStatement;

pub mod jql;
pub mod util;

/// Represents a field in JIRA, as returned by a [get fields request][1].
///
/// [1]: https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/#api-rest-api-3-field-get
#[derive(Debug, Deserialize)]
pub struct Field {
    pub id: String,

    pub name: String,
}

/// Represents an issue in JIRA, as returned by a [search request][1].
///
/// [1]: https://docs.atlassian.com/software/jira/docs/api/REST/9.6.0/#api/2/search-searchUsingSearchRequest
#[derive(Debug, Deserialize)]
pub struct SearchIssue {
    pub id: String,

    pub key: String,

    pub fields: HashMap<String, JSONValue>,
}

impl SearchIssue {
    /// Returns the statusCategory from the status field in an issue result.
    ///
    /// For this to appear in the list of fields, JIRA requires the `"status"` field to be passed into the search
    /// request body.  Fields make no guarantees about their typing, so we just reach into the JSON object directly
    /// rather than having a separate type for it.  If this returns `None` when you expect a value, check the body of
    /// the request to get issues to see if the status field is specified.
    pub fn status_category(&self) -> Option<String> {
        if let Some(status_obj) = self.fields.get("status") {
            let path = vec!["statusCategory", "name"];

            return util::get_string_in_json(status_obj, &path);
        }

        None
    }

    /// Returns an f64 from a field, if that is how the JSON is laid out.
    ///
    /// Useful for things like story point fields.
    pub fn numeric_field(&self, field_id: &str) -> Option<f64> {
        if let Some(JSONValue::Number(n)) = self.fields.get(field_id) {
            if let Some(n_f64) = n.as_f64() {
                return Some(n_f64);
            }
        }

        None
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    issues: Vec<SearchIssue>,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    fields: Vec<String>,

    jql: JQLStatement,

    #[serde(rename(serialize = "maxResults"))]
    max_results: u64,

    #[serde(rename(serialize = "startAt"))]
    start_at: u64,
}

#[derive(Clone, Debug)]
pub enum IssueEditUpdateLabel {
    Add(String),
    /* Remove would go here */
}

impl Serialize for IssueEditUpdateLabel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            IssueEditUpdateLabel::Add(label) => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("add", label)?;
                m.end()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IssueEditUpdate {
    pub labels: Vec<IssueEditUpdateLabel>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IssueEditRequest {
    pub update: IssueEditUpdate,
}

/// Provides a reusable HTTP client for using parts of JIRA's [V3 REST API][1].
///
/// It is currently suitable for my personal projects, and is not a complete implementation.  However, feel free to
/// extend this to meet your needs.
///
/// [1]: https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/
pub struct RestClient {
    base_url: String,
    client: Client,
}

impl RestClient {
    /// Initialize a RestClient for the URL, with the given username and token.
    ///
    /// This may fail if the TLS backend cannot be initialized, or if the resolver cannot load the system
    /// configuration.
    pub fn new(url: &str, username: &str, token: &str) -> Result<Self, reqwest::Error> {
        let base64_engine =
            GeneralPurpose::new(&base64::alphabet::URL_SAFE, GeneralPurposeConfig::new());

        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        Self::add_auth_header(&mut default_headers, &base64_engine, username, token);

        let client = ClientBuilder::new()
            .default_headers(default_headers)
            .build()?;

        Ok(RestClient {
            base_url: format!("{}/rest/api/3", url),
            client,
        })
    }

    /// Encodes the auth header according to JIRA's [REST API V3 conventions][1].
    ///
    /// [1]: https://developer.atlassian.com/cloud/jira/platform/basic-auth-for-rest-apis/
    fn add_auth_header(
        headers: &mut HeaderMap,
        base64_engine: &GeneralPurpose,
        username: &str,
        token: &str,
    ) {
        let encoded = base64_engine.encode(format!("{}:{}", username, token));
        // Unwrap here is considered safe since the method returns an error if the input is out of bounds, which would
        // have to be a bug in the base64 library.
        let mut auth_header_value =
            HeaderValue::from_str(format!("Basic {}", encoded).as_str()).unwrap();
        auth_header_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_header_value);
    }

    /// Make a GET request to the specified path, using the URL, username, & token configured for the client.
    ///
    /// Returns a `reqwest::RequestBuilder` so that you can use any method available in the reqwest library.
    fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}/{}", self.base_url, path))
    }

    /// Make a POST request to the specified path, using the URL, username, & token configured for the client.
    ///
    /// Returns a `reqwest::RequestBuilder` so that you can use any method available in the reqwest library.
    fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(format!("{}/{}", self.base_url, path))
    }

    /// Make a PUT request to the specified path, using the URL, username, & token configured for the client.
    ///
    /// Returns a `reqwest::RequestBuilder` so that you can use any method available in the reqwest library.
    fn put(&self, path: &str) -> RequestBuilder {
        self.client.put(format!("{}/{}", self.base_url, path))
    }

    /// Gets all configured fields for your JIRA instance.
    ///
    /// This is important because some critical functionality (story points, for example) are implemented as custom
    /// fields, so this call is needed to match the ones for your integration by name.
    ///
    /// See https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-fields/#api-rest-api-3-field-get
    pub fn get_fields(&self) -> Result<Vec<Field>, reqwest::Error> {
        let response = self.get("/field").send()?.error_for_status()?;
        let fields: Vec<Field> = response.json()?;

        Ok(fields)
    }

    /// Search JIRA for issues matching the given JQL statement.
    ///
    /// This calls the search endpoint without getting all pages; a more handy method may be `search_all`, which visits
    /// each page for you.
    ///
    /// See https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-search/#api-rest-api-3-search-post
    fn search(
        &self,
        fields: &Vec<String>,
        jql: &JQLStatement,
        start_at: u64,
        max_results: u64,
    ) -> Result<SearchResponse, reqwest::Error> {
        let response = self
            .post("/search")
            .json(&SearchRequest {
                fields: fields.to_vec(),
                jql: jql.clone(),
                start_at,
                max_results,
            })
            .send()?
            .error_for_status()?;
        response.json()
    }

    /// Search JIRA for issues matching the given JQL statement.
    ///
    /// This will get each page for you; it is handy if you want to avoid dealing with pagination in the result set.
    /// If having explicit pagination is helpful, try `search`.
    ///
    /// See https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issue-search/#api-rest-api-3-search-post
    pub fn search_all(
        &self,
        fields: &Vec<String>,
        jql: &JQLStatement,
    ) -> Result<Vec<SearchIssue>, reqwest::Error> {
        let mut start_at = 0u64;
        let max_results = 100u64;
        let mut result = vec![];

        loop {
            let mut response = self.search(fields, jql, start_at, max_results)?;
            let num_responses = response.issues.len() as u64;
            result.append(&mut response.issues);

            if num_responses < max_results {
                break;
            }

            start_at = start_at + num_responses
        }

        Ok(result)
    }

    /// Edits an issue.
    ///
    /// For now, this only supports the methods in the "update" key of the request, but could be extended.
    ///
    /// See https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/#api-rest-api-3-issue-issueidorkey-put
    pub fn edit_issue(&self, key: &str, update: &IssueEditUpdate) -> Result<(), reqwest::Error> {
        let path = format!("/issue/{}", key);
        let response = self
            .put(&path)
            .json(&IssueEditRequest {
                update: update.clone(),
            })
            .send()?
            .error_for_status()?;
        response.json()
    }
}
