use reqwest::{Client, Method, RequestBuilder};

pub struct Endpoint {
    client: Client,
    base: String,
}

impl Endpoint {
    pub fn new(url: &str) -> Result<Self, String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| format!("bad ELASTICSEARCH_URL: {e}"))?;
        Ok(Self {
            client: Client::builder()
                .build()
                .map_err(|e| format!("http client: {e}"))?,
            base: parsed.as_str().trim_end_matches('/').to_string(),
        })
    }

    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client.request(method, format!("{}{path}", self.base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn built(url: &str, path: &str) -> reqwest::Request {
        Endpoint::new(url)
            .expect("parses")
            .request(Method::GET, path)
            .build()
            .expect("a request")
    }

    fn authorization(request: &reqwest::Request) -> Option<&str> {
        request
            .headers()
            .get("authorization")
            .map(|value| value.to_str().expect("an ascii header"))
    }

    #[test]
    fn a_request_joins_the_path_and_carries_the_urls_credentials_as_basic_auth() {
        let request = built("http://reader:secret@es.internal:9200", "/poems/_search");
        assert_eq!(
            request.url().as_str(),
            "http://es.internal:9200/poems/_search"
        );
        assert_eq!(authorization(&request), Some("Basic cmVhZGVyOnNlY3JldA=="));
    }

    #[test]
    fn leaves_an_anonymous_url_unauthenticated() {
        assert_eq!(authorization(&built("http://localhost:9200", "/x")), None);
    }

    #[test]
    fn trims_a_trailing_slash_from_the_base() {
        assert_eq!(
            built("http://localhost:9200/", "/_cat").url().as_str(),
            "http://localhost:9200/_cat"
        );
    }

    #[test]
    fn a_password_free_userinfo_still_authenticates() {
        assert_eq!(
            authorization(&built("http://reader@localhost:9200", "/x")),
            Some("Basic cmVhZGVyOg==")
        );
    }

    #[test]
    fn a_percent_encoded_password_is_decoded_before_it_is_sent() {
        assert_eq!(
            authorization(&built("http://u:p%40ss@h:9200", "/x")),
            Some("Basic dTpwQHNz")
        );
    }

    #[test]
    fn a_password_without_a_username_still_authenticates() {
        assert_eq!(
            authorization(&built("http://:secret@h:9200", "/x")),
            Some("Basic OnNlY3JldA==")
        );
    }

    #[test]
    fn rejects_a_url_it_cannot_parse() {
        assert!(Endpoint::new("not a url").is_err());
    }

    #[test]
    fn a_base_with_a_path_keeps_it_without_the_trailing_slash() {
        assert_eq!(
            built("http://h/es/", "/_cat").url().as_str(),
            "http://h/es/_cat"
        );
    }
}
