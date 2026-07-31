//! HTTP Basic Authentication credentials.

/// HTTP Basic Authentication credentials.
///
/// Pass an instance via [`Client::with_basic_auth`](crate::Client::with_basic_auth)
/// to include an `Authorization: Basic <base64>` header on every request.
#[derive(Clone, PartialEq, Eq)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl BasicAuth {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl std::fmt::Debug for BasicAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicAuth")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stores_credentials() {
        let auth = BasicAuth::new("user", "pass");
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn debug_redacts_password() {
        let auth = BasicAuth::new("user", "s3cret");
        let rendered = format!("{auth:?}");
        assert!(rendered.contains("user"));
        assert!(!rendered.contains("s3cret"));
        assert!(rendered.contains("redacted"));
    }
}
