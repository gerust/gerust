use http;

use resource::Resource;

static DIAGRAM_VERSION: u8 = 3;

static START: States = States::B13;

#[derive(Debug, Clone, Copy)]
enum States {
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

struct Flow<R> where R: Resource {
    resource: R,
    state: States
}

impl<R, B> Flow<R> where R: Resource<Request=http::Request<B>, Response=http::Response<B>> {
    fn new(resource: R) -> Flow<R> {
        Flow { resource, state: States::default() }
    }

    fn transition(self) -> Result<Self, Outcomes> {
        let Flow { mut resource, state } = self;
        let next = match state {
            States::B13 => { B13::execute(&mut resource) },
            States::B12 => { B12::execute(&mut resource) },
            States::B11 => { B11::execute(&mut resource) },
            States::B10 => { B10::execute(&mut resource) }

            _ => { unimplemented!() },
        }?;

        Ok(Flow { resource, state: next })
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
            Ok(States::B10)
        } else {
            Err(Outcomes::Halt(http::StatusCode::URI_TOO_LONG))
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
                    Ok(States::B8)
                } else {
                    Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
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
                    Ok(States::B8)
                } else {
                    Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
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
            Ok(States::B6)
        } else {
            Err(Outcomes::Halt(http::StatusCode::FORBIDDEN))
        }
    }
}


