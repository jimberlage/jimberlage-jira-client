# jimberlage_jira_client

Provides a library for accessing JIRA.  While the intent is to write it in such a way as to make it support any aspect of the JIRA API, it is currently very limited in scope and probably only suitable for small projects.  Until it covers more of the JIRA API, it will remain prefixed with `jimberlage` on the crate so that you can opt for a more feature-rich JIRA client in the meantime.

If you need a more featureful client, try [jira](https://crates.io/crates/jira).  Based on OpenAPI, probably suitable if you need a lot of the JIRA API.

## Usage

```
cargo add jimberlage_jira_client
```

## Tests

Most tests in this repository are doc tests.  I unfortunately don't have a good way to do tests against the JIRA REST client, as that would require a dedicated public JIRA instance and personal access token.

```
cargo test
```
