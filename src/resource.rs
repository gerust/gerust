use std;
use http;
use mime;

pub trait Resource where Self: 'static {
    fn resource_exists(&self) -> bool {
        true
    }

    fn service_available(&self) -> bool {
        true
    }

    fn is_authorized(&self, _authorization_header: Option<&http::header::HeaderValue>) -> bool {
        true
    }

    fn forbidden(&self) -> bool {
        false
    }

    fn allow_missing_post(&self) -> bool {
        false
    }

    fn malformed_request(&self) -> bool {
        false
    }

    fn uri_too_long(&self, &http::uri::Uri) -> bool {
        false
    }

    fn known_content_type(&self, _content_type: &http::header::HeaderValue) -> bool {
        true
    }

    fn valid_content_headers<'a, I: Iterator<Item=(&'a http::header::HeaderName, &'a http::header::HeaderValue)>>(&self, _content_headers: I) -> bool {
        true
    }

    fn valid_entity_length(&self, _len: u64) -> bool {
        true
    }

    fn options(&self) -> &'static [http::header::HeaderValue] {
        &[]
    }

    fn allowed_methods(&self) -> &'static [http::Method] {
        &[http::Method::GET, http::Method::HEAD]
    }

    fn known_methods(&self) -> &'static [http::Method] {
        use http::Method;

        &[Method::GET, Method::HEAD, Method::POST, Method::PUT, Method::DELETE, Method::TRACE, Method::CONNECT, Method::OPTIONS]
    }

    fn delete_method(&self) -> bool {
        false
    }

    fn delete_completed(&self) -> bool {
        true
    }

    fn post_is_create(&self) -> bool {
        false
    }

    fn create_path(&self) {

    }

    fn base_uri(&self) -> Option<http::uri::Uri> {
        None
    }

    fn process_post(&self) -> bool {
        false
    }

    ///TODO: create handler interface
    fn content_types_provided(&self) -> &'static [(mime::Mime, fn (&mut Self, response: &mut ::flow::DelayedResponse) -> ())];

    ///TODO: create handler interface
    fn content_types_accepted(&self) -> &'static [(mime::Mime, fn (&mut Self, response: &mut ::flow::DelayedResponse) -> ())] {
        &[]
    }

    fn charsets_provided(&self) -> &'static [http::header::HeaderValue] {
        &[]
    }

    fn languages_provided(&self) -> &'static [&'static str] {
        &[]
    }

    ///TODO: create handler interface
    fn encodings_provided(&self) -> &'static [(&'static str, fn (&Self) -> ())] {
        &[]
        //&[("IDENTITY", encode_identity)]
    }

    fn variances(&self) -> &'static [http::header::HeaderValue] {
        &[]
    }

    fn is_conflict(&self) -> bool {
        false
    }

    fn multiple_choices(&self) -> bool {
        false
    }

    fn previously_existed(&self) -> bool {
        false
    }

    fn moved_permanently(&self) -> Option<http::uri::Uri> {
        None
    }

    fn moved_temporarily(&self) -> Option<http::uri::Uri> {
        None
    }

    /// TODO: Probably Chrono?
    fn last_modified(&self) -> Option<std::time::SystemTime> {
        None
    }

    fn generate_etag(&self) -> Option<http::header::HeaderValue> {
        None
    }

    fn finish_request(&self) {

    }

    /// TODO: currently unsure if I want to adopt this API
    fn handle_error(_e: Box<std::any::Any>) {

    }

    /// TODO: currently unsure if I want to adopt this API
    fn validate_content_checksum(&self) -> Option<bool> {
        None
    }
}