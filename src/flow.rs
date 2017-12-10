use http;
use hyper;
use futures;
use futures::Stream;
use futures::Sink;
use futures::Future;
use futures::future::err;
use futures::sync::oneshot::Sender;

use tokio_core::reactor::Handle;

use resource::Resource;

use std::fmt::Debug;
use std;

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
pub enum Outcomes {
    StartResponse,
    Halt(http::status::StatusCode)
}

// TODO: Maybe turn into struct, holding body and builder?
#[derive(Debug)]
pub enum DelayedResponse<B: Debug> {
    Waiting(http::response::Builder),
    Started(http::response::Response<B>)
}

impl<B: Debug> DelayedResponse<B> {
    fn new() -> DelayedResponse<B> {
        let builder = http::response::Builder::new();
        DelayedResponse::Waiting(builder)
    }

    fn builder(&mut self) -> &mut http::response::Builder {
        match *self {
            DelayedResponse::Waiting(ref mut b) => b,
            _ => { panic!("called builder() after response has started!") }
        }
    }

    fn response(&mut self) -> &mut http::response::Response<B> {
        match *self {
            DelayedResponse::Started(ref mut r) => r,
            _ => { panic!("called response() before response has started!") }
        }
    }
}

//TODO: Should be "ResourceWrapper"
#[derive(Debug)]
pub struct ResponseWrapper<R, B>
    where R: Resource {
    resource: R,
    request: http::Request<B>,
    response: DelayedResponse<hyper::Body>
}

impl<R, B> ResponseWrapper<R, B>
    where R: Resource
{
    fn new(resource: R, request: http::Request<B>) -> Self {
        let delay = DelayedResponse::new();

        ResponseWrapper { resource: resource, request: request, response: delay }
    }
}

impl Default for States {
    fn default() -> States {
        START
    }
}

trait State<R, B> where R: Resource {
    const LABEL: States;

    fn execute(response: &mut ResponseWrapper<R ,B>) -> Result<States, Outcomes>;
}

pub trait Flow {
    type Request;
    type Response;
    type Future;
    type Error;

    fn new() -> Self;

    fn execute<R>(&mut self, resource: R, request: Self::Request, sx: Sender<Self::Response>)
        where R: Resource + Debug;

    fn transition<R, B>(&mut self, response_wrapper: &mut ResponseWrapper<R, B>) -> Result<(), Outcomes>
        where R: Resource;
}

#[derive(Debug)]
pub struct HttpFlow {
    state: States,
//    handle: Handle
}

pub struct FlowError;

impl Flow for HttpFlow
{
    type Request = http::Request<hyper::Body>;
    type Response = http::Response<hyper::Body>;
    type Error = FlowError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn new() -> HttpFlow {
        HttpFlow { state: START }
    }

    fn execute<R>(&mut self, mut resource: R, request: Self::Request, sx: Sender<Self::Response>)
        where R: Resource + Debug
    {
        let mut wrapper = ResponseWrapper::new(resource, request);

        let mut status = None;

        loop {
            println!("transitioning from: {:?}", self);

            let retval = self.transition(&mut wrapper);

            match retval {
                Ok(()) => {
                    println!("transitioned into: {:?}", self);
                    continue;
                },
                Err(Outcomes::StartResponse) => {

                },
                Err(Outcomes::Halt(s)) => {
                    status = Some(s);
                    let response: http::Response<hyper::Body> = wrapper.response.builder().status(status.unwrap()).body("Hello World?".into()).unwrap();
                    sx.send(response);
                    break;
                }
                _ => {}
            };
        }


    }

    fn transition<R, B>(&mut self, response_wrapper: &mut ResponseWrapper<R, B>) -> Result<(), Outcomes>
        where R: Resource
    {
        // TODO: clean this renaming up
        let resource = response_wrapper;

        let next = match self.state {
            States::B13 => { B13::execute(resource) },
            States::B12 => { B12::execute(resource) },
            States::B11 => { B11::execute(resource) },
            States::B10 => { B10::execute(resource) },
            States::B9  => { B9::execute(resource) },
            States::B8  => { B8::execute(resource) },
            States::B7  => { B7::execute(resource) },
            States::B6  => { B6::execute(resource) },
            States::B5  => { B5::execute(resource) },
            States::B4  => { B4::execute(resource) },
            States::B3  => { B3::execute(resource) },
            States::C3  => { C3::execute(resource) },
            States::C4  => { C4::execute(resource) },
            States::D4  => { D4::execute(resource) },
            States::D5  => { D5::execute(resource) },
            States::E5  => { E5::execute(resource) },
            States::E6  => { E6::execute(resource) },
            States::F6  => { F6::execute(resource) },
            States::F7  => { F7::execute(resource) },
            States::G7  => { G7::execute(resource) },
            States::G8  => { G8::execute(resource) },
            States::G9  => { G9::execute(resource) },
            States::G11 => { G11::execute(resource) },
            States::H10 => { H10::execute(resource) },
            States::M16 => { M16::execute(resource) },
            States::N16 => { N16::execute(resource) },
            States::O16 => { O16::execute(resource) },
            States::O18 => { O18::execute(resource) },
            _ => { unimplemented!() },
        }?;

        self.state = next;

        Ok(())
    }
}

struct B13;

impl<R, B> State<R, B> for B13 where R: Resource {
    const LABEL: States = States::B13;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        if wrapper.resource.service_available() {
            Ok(States::B12)
        } else {
            Err(Outcomes::Halt(http::StatusCode::SERVICE_UNAVAILABLE))
        }
    }
}

struct B12;

impl<R, B> State<R, B> for B12 where R: Resource {
    const LABEL: States = States::B12;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;
        let request = &mut wrapper.request;

        if resource.known_methods().contains(request.method()) {
            Ok(States::B11)
        } else {
            Err(Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED))
        }
    }
}

struct B11;

impl<R, B> State<R, B> for B11 where R: Resource {
    const LABEL: States = States::B11;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        if resource.uri_too_long(request.uri()) {
            Err(Outcomes::Halt(http::StatusCode::URI_TOO_LONG))
        } else {
            Ok(States::B10)
        }
    }
}

struct B10;

impl<R, B> State<R, B> for B10 where R: Resource {
    const LABEL: States = States::B10;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;
        let builder = wrapper.response.builder();

        if resource.allowed_methods().contains(request.method()) {
            Ok(States::B9)
        } else {
            let header= http::header::HeaderValue::from_str(&resource.allowed_methods().iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ")).unwrap();

            builder.header(http::header::ACCEPT, header);

            Err(Outcomes::Halt(http::StatusCode::METHOD_NOT_ALLOWED))
        }
    }
}

struct B9;

impl<R, B> State<R, B> for B9 where R: Resource {
    const LABEL: States = States::B9;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;


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

impl<R, B> State<R, B> for B8 where R: Resource {
    const LABEL: States = States::B8;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let auth_header = request.headers().get(http::header::AUTHORIZATION);

        // TODO: Implement full is_authorized protocol
        if resource.is_authorized(auth_header) {
            Ok(States::B7)
        } else {
            Err(Outcomes::Halt(http::StatusCode::UNAUTHORIZED))
        }
    }
}

struct B7;

impl<R, B> State<R, B> for B7 where R: Resource {
    const LABEL: States = States::B7;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;


        if resource.forbidden() {
            Err(Outcomes::Halt(http::StatusCode::FORBIDDEN))
        } else {
            Ok(States::B6)
        }
    }
}

struct B6;

impl<R, B> State<R, B> for B6 where R: Resource {
    const LABEL: States = States::B6;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let headers = request.headers().iter()
            .filter(|&(name, _)| name.as_str().starts_with("CONTENT-"));

        if resource.valid_content_headers(headers) {
            Ok(States::B5)
        } else {
            Err(Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED))
        }
    }
}

struct B5;

impl<R, B> State<R, B> for B5 where R: Resource {
    const LABEL: States = States::B5;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let content_type = request.headers().get("Content-Type");

        let default = http::header::HeaderValue::from_str("application/octet-stream").unwrap();
        let ct = content_type.unwrap_or(&default);

        // TODO: Properly handle Content-Type not being given
        if resource.known_content_type(ct) {
            Ok(States::B4)
        } else {
            Err(Outcomes::Halt(http::StatusCode::UNSUPPORTED_MEDIA_TYPE))
        }
    }
}

struct B4;

impl<R, B> State<R, B> for B4 where R: Resource {
    const LABEL: States = States::B4;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        use http::method::Method;

        let content_length = request.headers().get("Content-Length");
        let transfer_encoding = request.headers().get("Transfer-Encoding");

        match *request.method() {
            Method::GET | Method::HEAD | Method::OPTIONS => {
                if content_length.is_some() {
                    // TODO: Communicate _why_ it is a BAD_REQUEST
                    return Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
                } else {
                    return Ok(States::B3)
                }
            },
            _ => {}
        }

        if transfer_encoding.is_some() && content_length.is_some() {
            // TODO: Communicate _why_ it is a BAD_REQUEST
            return Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
        }

        // TODO: Properly handle Content-Length not being given
        if let Some(cl) = content_length {
            if let Ok(stringed) = cl.to_str() {
                if let Ok(parsed) = stringed.parse() {
                    if resource.valid_entity_length(parsed) {
                        Ok(States::B3)
                    } else {
                        Err(Outcomes::Halt(http::StatusCode::PAYLOAD_TOO_LARGE))
                    }
                } else {
                    // TODO: Communicate _why_ it is a BAD_REQUEST
                    return Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
                }
            } else {
                // TODO: Communicate _why_ it is a BAD_REQUEST
                return Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
            }
        } else {
            // TODO: Communicate _why_ it is a BAD_REQUEST
            return Err(Outcomes::Halt(http::StatusCode::BAD_REQUEST))
        }

    }
}

struct B3;

impl<R, B> State<R, B> for B3 where R: Resource {
    const LABEL: States = States::B3;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let method = request.method();

        // Ugh, Result doesn't seem good here.
        if *method == http::method::Method::OPTIONS {
            Err(Outcomes::Halt(http::StatusCode::OK))
        } else {
            Ok(States::C3)
        }
    }
}


struct C3;

impl<R, B> State<R, B> for C3 where R: Resource {
    const LABEL: States = States::C3;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept = request.headers().get(http::header::ACCEPT);

        if let Some(header) = accept {
            Ok(States::C4)
        } else {
            Ok(States::D4)
        }
    }
}

struct C4;

impl<R, B> State<R, B> for C4 where R: Resource {
    const LABEL: States = States::C4;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept = request.headers().get(http::header::ACCEPT);

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

impl<R, B> State<R, B> for D4 where R: Resource {
    const LABEL: States = States::D4;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_language = request.headers().get(http::header::ACCEPT_LANGUAGE);

        if let Some(header) = accept_language {
            Ok(States::D5)
        } else {
            Ok(States::E5)
        }
    }
}

struct D5;

impl<R, B> State<R, B> for D5 where R: Resource {
    const LABEL: States = States::D5;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_language = request.headers().get(http::header::ACCEPT_LANGUAGE);

        if let Some(header) = accept_language {
            // TODO: this algorithm is too simple
            if resource.languages_provided().contains(&header.to_str().unwrap()) {
                Ok(States::E5)
            } else {
                Err(Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE))
            }
        } else {
            unreachable!()
        }
    }
}

struct E5;

impl<R, B> State<R, B> for E5 where R: Resource {
    const LABEL: States = States::D5;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_charset = request.headers().get(http::header::ACCEPT_CHARSET);

        if let Some(header) = accept_charset {
            Ok(States::E6)
        } else {
            Ok(States::F6)
        }
    }
}

struct E6;

impl<R, B> State<R, B> for E6 where R: Resource {
    const LABEL: States = States::E6;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_charset = request.headers().get(http::header::ACCEPT_CHARSET);

        if let Some(header) = accept_charset {
            // TODO: this algorithm is too simple
            if resource.charsets_provided().contains(&header) {
                Ok(States::G7)
            } else {
                Err(Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE))
            }
        } else {
            unreachable!()
        }
    }
}

struct F6;

impl<R, B> State<R, B> for F6 where R: Resource {
    const LABEL: States = States::F6;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_charset = request.headers().get(http::header::ACCEPT_CHARSET);

        if let Some(header) = accept_charset {
            Ok(States::F7)
        } else {
            Ok(States::G7)
        }
    }
}

struct F7;

impl<R, B> State<R, B> for F7 where R: Resource {
    const LABEL: States = States::F7;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let accept_encoding = request.headers().get(http::header::ACCEPT_ENCODING);

        if let Some(header) = accept_encoding {
            // TODO: this algorithm is too simple
            if true {
                Ok(States::G7)
            } else {
                Err(Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE))
            }
        } else {
            unreachable!()
        }
    }
}

struct G7;

impl<R, B> State<R, B> for G7 where R: Resource {
    const LABEL: States = States::G7;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;


        if resource.resource_exists() {
            Ok(States::G8)
        } else {
            Ok(States::H7)
        }
    }
}

struct G8;

impl<R, B> State<R, B> for G8 where R: Resource {
    const LABEL: States = States::G8;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let if_match = request.headers().get(http::header::IF_MATCH);

        if let Some(header) = if_match {
            Ok(States::G9)
        } else {
            Ok(States::H10)
        }
    }
}

struct G9;

impl<R, B> State<R, B> for G9 where R: Resource {
    const LABEL: States = States::G9;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let if_match = request.headers().get(http::header::IF_MATCH);

        if let Some(header) = if_match {
            if header.to_str().unwrap() == "*" {
                Ok(States::H10)
            } else {
                Ok(States::G11)
            }
        } else {
            unreachable!()
        }
    }
}

struct G11;

impl<R, B> State<R, B> for G11 where R: Resource {
    const LABEL: States = States::G11;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let resource = &mut wrapper.resource;

        let request = &mut wrapper.request;

        let if_match = request.headers().get(http::header::IF_MATCH);

        if let Some(header) = if_match {
            //TODO: Implement correctly
            let etag_in_if_match = true;
            if etag_in_if_match {
                Ok(States::H10)
            } else {
                Err(Outcomes::Halt(http::StatusCode::PRECONDITION_FAILED))
            }
        } else {
            unreachable!()
        }
    }
}

struct H10;

impl<R, B> State<R, B> for H10 where R: Resource {
    const LABEL: States = States::H10;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        // TODO: we currently just skip through
        Ok(States::M16)
    }
}

// TODO: CONDITION HANDLING

struct M16;

impl<R, B> State<R, B> for M16 where R: Resource {
    const LABEL: States = States::M16;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let request = &mut wrapper.request;

        if http::method::Method::DELETE == *request.method() {
            Ok(States::M20)
        } else {
            Ok(States::N16)
        }
    }
}

struct N16;

impl<R, B> State<R, B> for N16 where R: Resource {
    const LABEL: States = States::N16;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let request = &mut wrapper.request;

        if http::method::Method::POST == *request.method() {
            Ok(States::N11)
        } else {
            Ok(States::O16)
        }
    }
}


struct O16;

impl<R, B> State<R, B> for O16 where R: Resource {
    const LABEL: States = States::O16;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        let request = &mut wrapper.request;

        if http::method::Method::PUT == *request.method() {
            Ok(States::O14)
        } else {
            Ok(States::O18)
        }
    }
}

struct O18;

//impl<'a, R, B, BD> State<R> for O18
//    where R: Resource<Request=http::Request<BD>, Response=http::Response<BD>>,
//          R: Debug,
//          B: From<&'a [u8]>,
//          BD: Sink<SinkItem=B, SinkError=http::Error> + Debug,

impl<R, B> State<R, B> for O18 where R: Resource
{
    const LABEL: States = States::O18;

    fn execute(wrapper: &mut ResponseWrapper<R,B>) -> Result<States, Outcomes> {
        // TODO MULTIPLE CHOICES MISSING
//        resource.response_mut().body_mut().send("hello!".as_bytes().into());
        
        Err(Outcomes::Halt(http::StatusCode::OK))
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

