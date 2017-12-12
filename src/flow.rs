use http;
use hyper;
use mime;
use futures;
use futures::Sink;
use futures::Future;
use futures::sync::oneshot::Sender;

use resource::Resource;

use std::fmt::Debug;

pub static DIAGRAM_VERSION: u8 = 3;

pub enum Outcomes<R, B> where R: Resource {
    Next(fn(&mut ResourceWrapper<R,B>) -> Outcomes<R, B>),
    StartResponse,
    Handle(fn(&mut R, &mut http::Request<hyper::Body>, &mut DelayedResponse)),
    Halt(http::status::StatusCode),
}

// TODO: Maybe turn into struct,     holding body and builder?
pub enum DelayedResponse {
    Waiting(http::response::Builder),
    Started(futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>>)
}

impl DelayedResponse {
    fn new() -> DelayedResponse {
        let builder = http::response::Builder::new();
        DelayedResponse::Waiting(builder)
    }

    fn builder(&mut self) -> &mut http::response::Builder {
        match *self {
            DelayedResponse::Waiting(ref mut b) => b,
            _ => { panic!("called builder() after response has started!") }
        }
    }

    pub fn response_body(&mut self) -> &mut futures::sync::mpsc::Sender<Result<hyper::Chunk, hyper::Error>> {
        match *self {
            DelayedResponse::Started(ref mut r) => r,
            _ => { panic!("called response() before response has started!") }
        }
    }
}

pub trait Flow {
    type Request;
    type Response;
    type Future;
    type Error;

    fn new() -> Self;

    fn execute<R>(&mut self, resource: R, request: Self::Request, sx: Sender<Self::Response>)
        where R: Resource + Debug;
}

#[derive(Debug)]
pub struct HttpFlow;

pub struct FlowError;

impl Flow for HttpFlow
{
    type Request = http::Request<hyper::Body>;
    type Response = http::Response<hyper::Body>;
    type Error = FlowError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn new() -> HttpFlow {
        HttpFlow
    }

    fn execute<R>(&mut self, resource: R, request: Self::Request, sx: Sender<Self::Response>)
        where R: Resource + Debug
    {
        let mut wrapper = ResourceWrapper::new(resource, request);

        let mut current = Outcomes::Next(ResourceWrapper::b13);

        loop {
            //println!("transitioning from: {:?}", self);

            match current {
                Outcomes::Next(f) => {
                    //println!("transitioned into: {:?}", self);
                    current = f(&mut wrapper);
                    continue;
                },
                Outcomes::StartResponse => {
                    //println!("received StartResponse!");

                    let (sink, body) = hyper::Body::pair();
                    // TODO: Fail properly
                    let response = wrapper.response.builder().body(body).unwrap();
                    wrapper.response = DelayedResponse::Started(sink);
                    sx.send(response);
                    //println!("response started: {:?}", self);
                    break;
                },
                Outcomes::Handle(handler) => {
                    //println!("handling!");
                    let (sink, body) = hyper::Body::pair();
                    // TODO: Fail properly
                    let response = wrapper.response.builder().body(body).unwrap();
                    wrapper.response = DelayedResponse::Started(sink);
                    sx.send(response);
                    //println!("response started: {:?}", self);
                    handler(&mut wrapper.resource, &mut wrapper.request, &mut wrapper.response);

                    wrapper.response.response_body().poll_complete();
                    break;
                },
                Outcomes::Halt(s) => {
                    let response: http::Response<hyper::Body> = wrapper.response.builder().status(s).body(s.canonical_reason().unwrap().into()).unwrap();
                    sx.send(response);
                    break;
                }
            };
        }
    }
}

pub struct ResourceWrapper<R, B>
    where R: Resource {
    resource: R,
    pub request: http::Request<B>,
    response: DelayedResponse
}

impl<R, B> ResourceWrapper<R, B>
    where R: Resource
{
    fn new(resource: R, request: http::Request<B>) -> Self {
        let delay = DelayedResponse::new();

        ResourceWrapper { resource: resource, request: request, response: delay }
    }
}

impl<R, B> ResourceWrapper<R, B> where R: Resource {
    fn b13(&mut self) -> Outcomes<R, B> {
        if self.resource.service_available() {
            Outcomes::Next(Self::b12)
        } else {
            Outcomes::Halt(http::StatusCode::SERVICE_UNAVAILABLE)
        }
    }

    fn b12(&mut self) -> Outcomes<R, B> {
        if self.resource.known_methods().contains(self.request.method()) {
            Outcomes::Next(Self::b11)
        } else {
            Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED)
        }
    }

    fn b11(&mut self) -> Outcomes<R, B> {
        if self.resource.uri_too_long(self.request.uri()) {
            Outcomes::Halt(http::StatusCode::URI_TOO_LONG)
        } else {
            Outcomes::Next(Self::b10)
        }
    }

    fn b10(&mut self) -> Outcomes<R, B> {
        let builder = self.response.builder();

        if self.resource.allowed_methods().contains(self.request.method()) {
            Outcomes::Next(Self::b9)
        } else {
            let header = http::header::HeaderValue::from_str(&self.resource.allowed_methods().iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ")).unwrap();

            builder.header(http::header::ACCEPT, header);

            Outcomes::Halt(http::StatusCode::METHOD_NOT_ALLOWED)
        }
    }

    fn b9(&mut self) -> Outcomes<R, B> {
        if let Some(result) = self.resource.validate_content_checksum() {
            if result {
                if self.resource.malformed_request() {
                    Outcomes::Halt(http::StatusCode::BAD_REQUEST)
                } else {
                    Outcomes::Next(Self::b8)
                }
            } else {
                //resource.response_mut().body("Content-MD5 header does not match request body.")
                Outcomes::Halt(http::StatusCode::BAD_REQUEST)
            }
        } else {
            // TODO: MD5 validation of body
            let valid = true;
            if valid {
                if self.resource.malformed_request() {
                    Outcomes::Halt(http::StatusCode::BAD_REQUEST)
                } else {
                    Outcomes::Next(Self::b8)
                }
            } else {
                Outcomes::Halt(http::StatusCode::BAD_REQUEST)
            }
        }
    }

    fn b8(&mut self) -> Outcomes<R, B> {
        let auth_header = self.request.headers().get(http::header::AUTHORIZATION);

        // TODO: Implement full is_authorized protocol
        if self.resource.is_authorized(auth_header) {
            Outcomes::Next(Self::b7)
        } else {
            Outcomes::Halt(http::StatusCode::UNAUTHORIZED)
        }
    }

    fn b7(&mut self) -> Outcomes<R, B> {
        if self.resource.forbidden() {
            Outcomes::Halt(http::StatusCode::FORBIDDEN)
        } else {
            Outcomes::Next(Self::b6)
        }
    }

    fn b6(&mut self) -> Outcomes<R, B> {
        let headers = self.request.headers().iter()
            .filter(|&(name, _)| name.as_str().starts_with("CONTENT-"));

        if self.resource.valid_content_headers(headers) {
            Outcomes::Next(Self::b5)
        } else {
            Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED)
        }
    }

    fn b5(&mut self) -> Outcomes<R, B> {
        let content_type = self.request.headers().get("Content-Type");

        // Default Content-Type is application/octet-stream. https://www.w3.org/Protocols/rfc2616/rfc2616-sec7.html#sec7.2.1
        let default = http::header::HeaderValue::from_str("application/octet-stream").unwrap();
        let ct = content_type.unwrap_or(&default);

        if self.resource.known_content_type(ct) {
            Outcomes::Next(Self::b4)
        } else {
            Outcomes::Halt(http::StatusCode::UNSUPPORTED_MEDIA_TYPE)
        }
    }

    fn b4(&mut self) -> Outcomes<R, B> {
        use http::method::Method;

        let content_length = self.request.headers().get("Content-Length");
        let transfer_encoding = self.request.headers().get("Transfer-Encoding");

        match *self.request.method() {
            Method::GET | Method::HEAD | Method::OPTIONS => {
                if content_length.is_some() {
                    // TODO: Communicate _why_ it is a BAD_REQUEST
                    return Outcomes::Halt(http::StatusCode::BAD_REQUEST)
                } else {
                    return Outcomes::Next(Self::b3)
                }
            },
            _ => {}
        }

        if transfer_encoding.is_some() && content_length.is_some() {
            // TODO: Communicate _why_ it is a BAD_REQUEST
            return Outcomes::Halt(http::StatusCode::BAD_REQUEST)
        }

        if let Some(cl) = content_length {
            if let Ok(stringed) = cl.to_str() {
                if let Ok(parsed) = stringed.parse() {
                    if self.resource.valid_entity_length(parsed) {
                        Outcomes::Next(Self::b3)
                    } else {
                        Outcomes::Halt(http::StatusCode::PAYLOAD_TOO_LARGE)
                    }
                } else {
                    // TODO: Communicate _why_ it is a BAD_REQUEST
                    Outcomes::Halt(http::StatusCode::BAD_REQUEST)
                }
            } else {
                // TODO: Communicate _why_ it is a BAD_REQUEST
                Outcomes::Halt(http::StatusCode::BAD_REQUEST)
            }
        } else {
            // TODO: Communicate _why_ it is a BAD_REQUEST
            Outcomes::Halt(http::StatusCode::BAD_REQUEST)
        }
    }

    fn b3(&mut self) -> Outcomes<R, B> {
        let method = self.request.method();

        if *method == http::method::Method::OPTIONS {
            Outcomes::Halt(http::StatusCode::OK)
        } else {
            Outcomes::Next(Self::c3)
        }
    }

    fn c3(&mut self) -> Outcomes<R, B> {
        let accept = self.request.headers().get(http::header::ACCEPT);

        let next = if accept.is_some() {
            Self::c4
        } else {
            Self::d4
        };

        Outcomes::Next(next)
    }

    fn c4(&mut self) -> Outcomes<R, B> {
        let accept = self.request.headers().get(http::header::ACCEPT);

        if let Some(_header) = accept {
            // TODO actually choose the type
            let chosen_type = true;

            if chosen_type {
                Outcomes::Next(Self::d4)
            } else {
                Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE)
            }
        } else {
            unreachable!();
        }
    }

    fn d4(&mut self) -> Outcomes<R, B> {
        let accept_language = self.request.headers().get(http::header::ACCEPT_LANGUAGE);

        let next = if accept_language.is_some() {
            Self::d5
        } else {
            Self::e5
        };

        Outcomes::Next(next)
    }

    fn d5(&mut self) -> Outcomes<R, B> {
        let accept_language = self.request.headers().get(http::header::ACCEPT_LANGUAGE);

        if let Some(header) = accept_language {
            // TODO: this algorithm is too simple
            if self.resource.languages_provided().contains(&header.to_str().unwrap()) {
                Outcomes::Next(Self::e5)
            } else {
                Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE)
            }
        } else {
            unreachable!()
        }
    }

    fn e5(&mut self) -> Outcomes<R, B> {
        let accept_charset = self.request.headers().get(http::header::ACCEPT_CHARSET);

        let next = if accept_charset.is_some() {
            Self::e6
        } else {
            Self::f6
        };

        Outcomes::Next(next)
    }

    fn e6(&mut self) -> Outcomes<R, B> {
        let accept_charset = self.request.headers().get(http::header::ACCEPT_CHARSET);

        if let Some(header) = accept_charset {
            // TODO: this algorithm is too simple
            if self.resource.charsets_provided().contains(header) {
                Outcomes::Next(Self::g7)
            } else {
                Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE)
            }
        } else {
            unreachable!()
        }
    }

    fn f6(&mut self) -> Outcomes<R, B> {
        let accept_charset = self.request.headers().get(http::header::ACCEPT_CHARSET);

        let next = if accept_charset.is_some() {
            Self::f7
        } else {
            Self::g7
        };

        Outcomes::Next(next)
    }


    fn f7(&mut self) -> Outcomes<R, B> {
        let accept_encoding = self.request.headers().get(http::header::ACCEPT_ENCODING);

        if let Some(_header) = accept_encoding {
            // TODO: this algorithm is too simple
            if true {
                Outcomes::Next(Self::g7)
            } else {
                Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE)
            }
        } else {
            unreachable!()
        }
    }

    fn g7(&mut self) -> Outcomes<R, B> {
        let next = if self.resource.resource_exists() {
            Self::g8
        } else {
            unimplemented!() //Self::h7
        };

        Outcomes::Next(next)
    }

    fn g8(&mut self) -> Outcomes<R, B> {
        let if_match = self.request.headers().get(http::header::IF_MATCH);

        let next = if let Some(_header) = if_match {
            Self::g9
        } else {
            Self::h10
        };

        Outcomes::Next(next)
    }

    fn g9(&mut self) -> Outcomes<R, B> {
        let if_match = self.request.headers().get(http::header::IF_MATCH);

        if let Some(header) = if_match {
            let next = if header.to_str().unwrap() == "*" {
                Self::h10
            } else {
                Self::g11
            };

            Outcomes::Next(next)
        } else {
            unreachable!()
        }
    }

    fn g11(&mut self) -> Outcomes<R, B> {
        let if_match = self.request.headers().get(http::header::IF_MATCH);

        if let Some(_header) = if_match {
            //TODO: Implement correctly
            let etag_in_if_match = true;
            if etag_in_if_match {
                Outcomes::Next(Self::h10)
            } else {
                Outcomes::Halt(http::StatusCode::PRECONDITION_FAILED)
            }
        } else {
            unreachable!()
        }
    }

    fn h10(&mut self) -> Outcomes<R, B> {
        // TODO: we currently just skip through
        Outcomes::Next(Self::m16)
    }

    // TODO: CONDITION HANDLING

    fn m16(&mut self) -> Outcomes<R, B> {
        let next = if http::method::Method::DELETE == *self.request.method() {
            unimplemented!() //Self::m20
        } else {
            Self::n16
        };

        Outcomes::Next(next)
    }

    fn n16(&mut self) -> Outcomes<R, B> {
        let next = if http::method::Method::POST == *self.request.method() {
            unimplemented!() //Self::n11
        } else {
            Self::o16
        };

        Outcomes::Next(next)
    }

    fn o16(&mut self) -> Outcomes<R, B> {
        let next = if http::method::Method::PUT == *self.request.method() {
            unimplemented!() //Self::o14
        } else {
            Self::o18
        };

        Outcomes::Next(next)
    }

    fn o18(&mut self) -> Outcomes<R, B> {
        self.response.builder().status(200);

        let content_type = self.request.headers().get("Content-Type");

        if let Some(ct) = content_type {
            let mime: mime::Mime = ct.to_str().unwrap().parse().unwrap();
            let pair = self.resource.content_types_accepted().iter().find(|&&(ref m, _)| *m == mime);

            if let Some(&(_, handler)) = pair {
                Outcomes::Handle(handler)
            } else {
                panic!("No handler for content type.")
            }
        } else {
            Outcomes::StartResponse
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http;
    use resource::Resource;
    use mime;
    use hyper;
    use futures::sync::oneshot;

    #[derive(Default, Debug)]
    struct DefaultResource;

    impl Resource for DefaultResource  {
        fn content_types_accepted(&self) -> &'static  [(mime::Mime, fn (&mut Self, response: &mut ::flow::DelayedResponse) -> ())] {
            &[]
        }

        fn content_types_provided(&self) -> &'static [(mime::Mime, fn(&mut Self, response: &mut ::flow::DelayedResponse) -> ())] {
            &[(mime::TEXT_HTML, Self::to_html)]
        }
    }

    impl DefaultResource {
        fn to_html(&mut self, response: &mut ::flow::DelayedResponse) -> () {

        }
    }


    #[test]
    fn default() {
        let resource = DefaultResource::default();

        let mut flow = HttpFlow::new();

        let req = http::request::Builder::new()
            .method(http::method::Method::GET)
            .body("".into())
            .unwrap();

        let (sx, rx): (_, _) = oneshot::channel::<http::Response<hyper::Body>>();

        flow.execute(resource, req, sx);
    }
}

