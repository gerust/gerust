use http;

use resource::Resource;

use std::fmt::Debug;

static DIAGRAM_VERSION: u8 = 3;

static START: States = States::B13;

#[derive(Debug, Clone, Copy)]
pub enum States {
    B13,
    B12,
    B11,
    B10,
    B9,
    B8,
    B7,
    B6,
    B5,
    B4,
    B3,
    C3,
    C4,
    D4,
    D5,
    E5,
    E6,
    F6,
    F7,
    G7,
    G8,
    G9,
    G11,
    H7,
    H10,
    H11,
    H12,
    I4,
    I7,
    I12,
    I13,
    J18,
    K5,
    K7,
    K13,
    L5,
    L7,
    L13,
    L14,
    L15,
    L17,
    M5,
    M7,
    M16,
    M20,
    N5,
    N11,
    N16,
    O14,
    O16,
    O18,
    O20,
    P3,
    P11
}

#[derive(Debug)]
enum Outcomes {
    Halt(http::status::StatusCode)
}

impl Default for States {
    fn default() -> States {
        START
    }
}

trait State<R> where R: Resource {
    const LABEL: States;

    fn execute(resource: &mut R) -> Result<States, Outcomes>;
}

#[derive(Debug)]
pub struct Flow<R> where R: Resource + Debug {
    resource: R,
    state: States
}

impl<R, B> Flow<R>
    where R: Resource<Request=http::Request<B>, Response=http::Response<B>>,
          R: Debug
{
    pub fn new(resource: R) -> Flow<R> {
        Flow { resource, state: States::default() }
    }

    pub fn finish(self) -> R {
        let Flow { resource, state } = self;

        resource
    }

    fn transition(&mut self) -> Result<(), Outcomes> {
        let resource = &mut self.resource;

        let next = match self.state {
            States::B13 => { B13::execute(resource) },
            States::B12 => { B12::execute(resource) },
            States::B11 => { B11::execute(resource) },
            States::B10 => { B10::execute(resource) },
            States::B9 => { B9::execute(resource) },
            States::B8 => { B8::execute(resource) },
            States::B7 => { B7::execute(resource) },
            States::B6 => { B6::execute(resource) },
            States::B5 => { B5::execute(resource) },
            States::B4 => { B4::execute(resource) },
            States::B3 => { B3::execute(resource) },
            States::C3 => { C3::execute(resource) },
            States::C4 => { C4::execute(resource) },
            States::D4 => { D4::execute(resource) },
            _ => { unimplemented!() },
        }?;

        self.state = next;

        Ok(())
    }

    pub fn execute(&mut self) {
        println!("transitioning from: {:?}", self);

        match self.transition() {
            Ok(()) => {
                println!("transitioned into: {:?}", self);
                self.execute()
            },
            Err(e) => println!("Error or end: {:?}", e)
        }
    }
}

struct B13;

impl<R, B> State<R> for B13 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B13;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if resource.service_available() {
            Ok(States::B12)
        } else {
            Err(Outcomes::Halt(http::StatusCode::SERVICE_UNAVAILABLE))
        }
    }
}

struct B12;

impl<R, B> State<R> for B12 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B12;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if resource.known_methods().contains(resource.request().method()) {
            Ok(States::B11)
        } else {
            Err(Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED))
        }
    }
}

struct B11;

impl<R, B> State<R> for B11 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B11;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if resource.uri_too_long(resource.request().uri()) {
            Err(Outcomes::Halt(http::StatusCode::URI_TOO_LONG))
        } else {
            Ok(States::B10)
        }
    }
}

struct B10;

impl<R, B> State<R> for B10 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B10;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if resource.allowed_methods().contains(resource.request().method()) {
            Ok(States::B9)
        } else {
            let header= http::header::HeaderValue::from_str(&resource.allowed_methods().iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ")).unwrap();

            resource.response_mut().headers_mut()
                .insert(http::header::ACCEPT, header);

            Err(Outcomes::Halt(http::StatusCode::METHOD_NOT_ALLOWED))
        }
    }
}

struct B9;

impl<R, B> State<R> for B9 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B9;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if let Some(result) = resource.validate_content_checksum() {
            if result {
                if resource.malformed_request() {
                    Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
                } else {
                    Ok(States::B8)
                }
            } else {
                //resource.response_mut().body("Content-MD5 header does not match request body.")
                Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
            }
        } else {
            // TODO: MD5 validation of body
            let valid = true;
            if valid {
                if resource.malformed_request() {
                    Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
                } else {
                    Ok(States::B8)
                }
            } else {
                Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
            }
        }
    }
}

struct B8;

impl<R, B> State<R> for B8 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B8;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let auth_header = resource.request().headers().get(http::header::AUTHORIZATION);

        // TODO: Implement full is_authorized protocol
        if resource.is_authorized(auth_header) {
            Ok(States::B7)
        } else {
            Err(Outcomes::Halt(http::StatusCode::UNAUTHORIZED))
        }
    }
}

struct B7;

impl<R, B> State<R> for B7 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B7;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        if resource.forbidden() {
            Err(Outcomes::Halt(http::StatusCode::FORBIDDEN))
        } else {
            Ok(States::B6)
        }
    }
}

struct B6;

impl<R, B> State<R> for B6 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B6;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let headers = resource.request().headers().iter()
            .filter(|&(name, _)| name.as_str().starts_with("CONTENT-"));

        if resource.valid_content_headers(headers) {
            Ok(States::B5)
        } else {
            Err(Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED))
        }
    }
}

struct B5;

impl<R, B> State<R> for B5 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B5;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let content_type = resource.request().headers().get("Content-Type");

        // TODO: Properly handle Content-Type not being given
        if resource.known_content_type(content_type.unwrap()) {
            Ok(States::B4)
        } else {
            Err(Outcomes::Halt(http::StatusCode::UNSUPPORTED_MEDIA_TYPE))
        }
    }
}

struct B4;

impl<R, B> State<R> for B4 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B4;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let content_length = resource.request().headers().get("Content-Length");

        // TODO: Properly handle Content-Length not being given
        if resource.valid_entity_length(content_length.unwrap().to_str().unwrap().parse().unwrap()) {
            Ok(States::B4)
        } else {
            Err(Outcomes::Halt(http::StatusCode::PAYLOAD_TOO_LARGE))
        }
    }
}

struct B3;

impl<R, B> State<R> for B3 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::B3;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let method = resource.request().method();

        // Ugh, Result doesn't seem good here.
        if *method == http::method::Method::OPTIONS {
            Err(Outcomes::Halt(http::StatusCode::OK))
        } else {
            Ok(States::C3)
        }
    }
}


struct C3;

impl<R, B> State<R> for C3 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::C3;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let accept = resource.request().headers().get("Accept");

        if let Some(header) = accept {
            Ok(States::C4)
        } else {
            Ok(States::D4)
        }
    }
}

struct C4;

impl<R, B> State<R> for C4 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::C4;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        let accept = resource.request().headers().get("Accept");

        if let Some(header) = accept {
            // TODO actually choose the type
            let chosen_type = true;

            if chosen_type {
                Ok(States::D4)
            } else {
                Err(Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE))
            }
        } else {
            unreachable!();
        }
    }
}

struct D4;

impl<R, B> State<R> for D4 where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    const LABEL: States = States::D4;

    fn execute(resource: &mut R) -> Result<States, Outcomes> {
        unimplemented!("congratulations, you reached D4");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http;
    use resource::Resource;
    use mime;

    #[derive(Default, Debug)]
    struct DefaultResource<B> where B: Default {
        request: http::Request<B>,
        response: http::Response<B>
    }

    impl<B> Resource for DefaultResource<B> where B: Default {
        type Request = http::Request<B>;
        type Response = http::Response<B>;

        fn request(&self) -> &Self::Request {
            &self.request
        }

        fn request_mut(&mut self) -> &mut Self::Request {
            &mut self.request
        }

        fn response(&self) -> &Self::Response {
            &self.response
        }

        fn response_mut(&mut self) -> &mut Self::Response {
            &mut self.response
        }

        fn content_types_allowed(&self) -> &'static [(mime::Mime, fn(&Self) -> ())] {
            &[(mime::TEXT_HTML, default_html::<B>)]
        }
    }

    fn default_html<B: Default>(resource: &DefaultResource<B>) -> () {

    }

    #[test]
    fn default() {
        let resource: DefaultResource<Vec<u8>> = DefaultResource::default();

        let flow = Flow::new(resource);

        flow.execute();
    }
}

