use http;
use hyper;
use mime;
use backtrace;
use std;
use futures;
use futures::Sink;
use futures::Future;
use futures::sync::oneshot::Sender;
use ::Body;

use resource::Resource;

use std::fmt::Debug;

pub static DIAGRAM_VERSION: u8 = 3;

type StateFn<R> = fn(&mut ResourceWrapper<R>) -> Outcomes<R>;

pub enum Outcomes<R> where R: Resource {
    Next(StateFn<R>),
    StartResponse(StateFn<R>),
    Done,
    InputHandler(fn(&mut R, &mut http::Request<Body>, &mut DelayedResponse)),
    OutputHandler(fn(&mut R, &mut DelayedResponse)),
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
    type Request = http::Request<Body>;
    type Response = http::Response<Body>;
    type Error = FlowError;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn new() -> HttpFlow {
        HttpFlow
    }

    fn execute<R>(&mut self, resource: R, request: Self::Request, sx: Sender<Self::Response>)
        where R: Resource + Debug
    {
        let mut sender = Some(sx);

        let mut wrapper = ResourceWrapper::new(resource, request);

        let mut current = Outcomes::Next(ResourceWrapper::b13);

        loop {
            //println!("transitioning from: {:?}", self);

            match current {
                Outcomes::Next(f) => {
                    backtrace::resolve(f as *mut std::os::raw::c_void, |symbol| {
                        println!("transitioned into: {:?}", symbol);
                    });
                    current = f(&mut wrapper);
                    continue;
                },
                Outcomes::StartResponse(f) => {
                    backtrace::resolve(f as *mut std::os::raw::c_void, |symbol| {
                        println!("transitioned into: {:?}", symbol);
                    });
                    //println!("received StartResponse!");

                    let (sink, body) = Body::pair();
                    // TODO: Fail properly
                    let response = wrapper.response.builder().body(body).unwrap();
                    wrapper.response = DelayedResponse::Started(sink);
                    // TODO: Fail properly
                    sender.take().unwrap().send(response);
                    //println!("response started: {:?}", self);
                    current = f(&mut wrapper);
                },
                Outcomes::Done => {
                    //println!("received StartResponse!");

                    // TODO: Fail properly
                    let response = wrapper.response.builder().body("".into()).unwrap();

                    // TODO: Fail properly
                    sender.take().unwrap().send(response);
                    break;
                },
                outcome @ Outcomes::InputHandler(_) | outcome @ Outcomes::OutputHandler(_) => {
                    //println!("handling!");
                    let (sink, body) = Body::pair();
                    // TODO: Fail properly
                    let response = wrapper.response.builder().body(body).unwrap();
                    wrapper.response = DelayedResponse::Started(sink);
                    // TODO: Fail properly
                    sender.take().unwrap().send(response);
                    //println!("response started: {:?}", self);
                    match outcome {
                        Outcomes::InputHandler(handler) => {
                            handler(&mut wrapper.resource, &mut wrapper.request, &mut wrapper.response);
                        },
                        Outcomes::OutputHandler(handler) => {
                            handler(&mut wrapper.resource, &mut wrapper.response);
                        },
                        _ => { unreachable!() }
                    }

                    wrapper.response.response_body().poll_complete();
                    break;
                },
                Outcomes::Halt(s) => {
                    let response: http::Response<Body> = wrapper.response.builder().status(s).body(s.canonical_reason().unwrap().into()).unwrap();
                    // TODO: Fail properly
                    sender.take().unwrap().send(response);
                    break;
                }
            };
        }
    }
}

pub struct Metadata {
    content_type: Option<mime::Mime>,
}

pub struct ResourceWrapper<R>
    where R: Resource {
    resource: R,
    pub request: http::Request<Body>,
    response: DelayedResponse,
    metadata: Metadata
}

impl<R> ResourceWrapper<R>
    where R: Resource
{
    fn new(resource: R, request: http::Request<Body>) -> Self {
        let delay = DelayedResponse::new();
        let metadata = Metadata { content_type: None };

        ResourceWrapper { resource: resource, request: request, response: delay, metadata: metadata }
    }
}

impl<R> ResourceWrapper<R> where R: Resource {
    fn b13(&mut self) -> Outcomes<R> {
        if self.resource.service_available() {
            Outcomes::Next(Self::b12)
        } else {
            Outcomes::Halt(http::StatusCode::SERVICE_UNAVAILABLE)
        }
    }

    fn b12(&mut self) -> Outcomes<R> {
        if self.resource.known_methods().contains(self.request.method()) {
            Outcomes::Next(Self::b11)
        } else {
            Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED)
        }
    }

    fn b11(&mut self) -> Outcomes<R> {
        if self.resource.uri_too_long(self.request.uri()) {
            Outcomes::Halt(http::StatusCode::URI_TOO_LONG)
        } else {
            Outcomes::Next(Self::b10)
        }
    }

    fn b10(&mut self) -> Outcomes<R> {
        let builder = self.response.builder();

        if self.resource.allowed_methods().contains(self.request.method()) {
            Outcomes::Next(Self::b9)
        } else {
            let header = http::header::HeaderValue::from_str(&self.resource.allowed_methods().iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ")).unwrap();

            builder.header(http::header::ACCEPT, header);

            Outcomes::Halt(http::StatusCode::METHOD_NOT_ALLOWED)
        }
    }

    fn b9(&mut self) -> Outcomes<R> {
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

    fn b8(&mut self) -> Outcomes<R> {
        let auth_header = self.request.headers().get(http::header::AUTHORIZATION);

        // TODO: Implement full is_authorized protocol
        if self.resource.is_authorized(auth_header) {
            Outcomes::Next(Self::b7)
        } else {
            Outcomes::Halt(http::StatusCode::UNAUTHORIZED)
        }
    }

    fn b7(&mut self) -> Outcomes<R> {
        if self.resource.forbidden() {
            Outcomes::Halt(http::StatusCode::FORBIDDEN)
        } else {
            Outcomes::Next(Self::b6)
        }
    }

    fn b6(&mut self) -> Outcomes<R> {
        let headers = self.request.headers().iter()
            .filter(|&(name, _)| name.as_str().starts_with("CONTENT-"));

        if self.resource.valid_content_headers(headers) {
            Outcomes::Next(Self::b5)
        } else {
            Outcomes::Halt(http::StatusCode::NOT_IMPLEMENTED)
        }
    }

    fn b5(&mut self) -> Outcomes<R> {
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

    fn b4(&mut self) -> Outcomes<R> {
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

    fn b3(&mut self) -> Outcomes<R> {
        let method = self.request.method();

        if *method == http::method::Method::OPTIONS {
            Outcomes::Halt(http::StatusCode::OK)
        } else {
            Outcomes::Next(Self::c3)
        }
    }

    fn c3(&mut self) -> Outcomes<R> {
        let accept = self.request.headers().get(http::header::ACCEPT);

        let next = if accept.is_some() {
            Self::c4
        } else {
            // TODO: Proper error handling
            self.metadata.content_type = Some(self.resource.content_types_provided().first().unwrap().0.clone());

            Self::d4
        };

        Outcomes::Next(next)
    }

    fn c4(&mut self) -> Outcomes<R> {
        let accept = self.request.headers().get(http::header::ACCEPT);

        if let Some(header) = accept {
            let chosen_type = ::conneg::choose_mediatype(self.resource.content_types_provided(), header);

            match chosen_type {
                Ok(mime) => {
                    self.metadata.content_type = Some(mime.clone());

                    Outcomes::Next(Self::d4)
                },
                Err(::conneg::Error::NotProvided) => Outcomes::Halt(http::StatusCode::NOT_ACCEPTABLE),
                // TODO: Communicate _why_ it is a BAD_REQUEST
                Err(::conneg::Error::ParseError) => Outcomes::Halt(http::StatusCode::BAD_REQUEST),
            }
        } else {
            unreachable!();
        }
    }

    fn d4(&mut self) -> Outcomes<R> {
        let accept_language = self.request.headers().get(http::header::ACCEPT_LANGUAGE);

        let next = if accept_language.is_some() {
            Self::d5
        } else {
            Self::e5
        };

        Outcomes::Next(next)
    }

    fn d5(&mut self) -> Outcomes<R> {
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

    fn e5(&mut self) -> Outcomes<R> {
        let accept_charset = self.request.headers().get(http::header::ACCEPT_CHARSET);

        let next = if accept_charset.is_some() {
            Self::e6
        } else {
            Self::f6
        };

        Outcomes::Next(next)
    }

    fn e6(&mut self) -> Outcomes<R> {
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

    fn f6(&mut self) -> Outcomes<R> {
        let accept_charset = self.request.headers().get(http::header::ACCEPT_CHARSET);

        let next = if accept_charset.is_some() {
            Self::f7
        } else {
            Self::g7
        };

        Outcomes::Next(next)
    }


    fn f7(&mut self) -> Outcomes<R> {
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

    fn g7(&mut self) -> Outcomes<R> {
        let next = if self.resource.resource_exists() {
            Self::g8
        } else {
            unimplemented!() //Self::h7
        };

        Outcomes::Next(next)
    }

    fn g8(&mut self) -> Outcomes<R> {
        let if_match = self.request.headers().get(http::header::IF_MATCH);

        let next = if let Some(_header) = if_match {
            Self::g9
        } else {
            Self::h10
        };

        Outcomes::Next(next)
    }

    fn g9(&mut self) -> Outcomes<R> {
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

    fn g11(&mut self) -> Outcomes<R> {
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

    fn h10(&mut self) -> Outcomes<R> {
        // TODO: we currently just skip through
        Outcomes::Next(Self::m16)
    }

    // TODO: CONDITION HANDLING

    fn m16(&mut self) -> Outcomes<R> {
        let next = if http::method::Method::DELETE == *self.request.method() {
            unimplemented!() //Self::m20
        } else {
            Self::n16
        };

        Outcomes::Next(next)
    }
//    base_uri = resource.base_uri || request.base_uri
//    new_uri = URI.join(base_uri.to_s, uri)
//    request.disp_path = new_uri.path
//    response.headers[LOCATION] = new_uri.to_s
//    result = accept_helper
    fn n11(&mut self) -> Outcomes<R> {
        if self.resource.post_is_create() {
            let mime: mime::Mime = {
                let content_type = self.request.headers().get("Content-Type");
                if let Some(ct) = content_type {
                    ct.to_str().unwrap().parse().unwrap()
                } else {
                    // TODO handle parse errors correctly
                    unimplemented!()
                }
            };

            let pair = self.resource.content_types_accepted().iter().find(|&&(ref m, _)| *m == mime);

            if let Some(&(_, handler)) = pair {
                // TODO remove this very nasty hack
                //let any = &mut self.request as &mut std::any::Any;

                //let request = any.downcast_mut::<http::Request<Body>>().unwrap();

                handler(&mut self.resource, &mut self.request, &mut self.response);

                if self.resource.process_post(&mut self.response) {
                    self.response.builder()
                        .status(http::status::StatusCode::CREATED)
                        .header(http::header::LOCATION, &*self.resource.create_path());

                    Outcomes::Done
                } else {
                    unimplemented!(); //Outcomes::Next(Self::o20)
                }
            } else {
                unimplemented!();
            }
        } else {
            if self.resource.process_post(&mut self.response) {
                Outcomes::Halt(http::status::StatusCode::CREATED)
            } else {
                unimplemented!(); //Outcomes::Next(Self::o20)
            }
        }
    }

    fn n16(&mut self) -> Outcomes<R> {
        let next = if http::method::Method::POST == *self.request.method() {
            Self::n11
        } else {
            Self::o16
        };

        Outcomes::Next(next)
    }

    fn o16(&mut self) -> Outcomes<R> {
        let next = if http::method::Method::PUT == *self.request.method() {
            unimplemented!() //Self::o14
        } else {
            Self::o18
        };

        Outcomes::Next(next)
    }

    fn o18(&mut self) -> Outcomes<R> {
        self.response.builder().status(200);

        let content_type = self.metadata.content_type.as_ref();

        if let Some(mime) = content_type {
            let pair = self.resource.content_types_provided().iter().find(|&&::resource::ProvidedPair(ref m, _)| m == mime);

            if let Some(&::resource::ProvidedPair(_, handler)) = pair {
                Outcomes::OutputHandler(handler)
            } else {
                panic!("No handler for content type.")
            }
        } else {
            unimplemented!();
        }
    }
}


