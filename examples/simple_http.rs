#[macro_use]
extern crate log;
extern crate env_logger;

extern crate hyper;

extern crate gerust;
extern crate http;
extern crate mime;

use gerust::resource::Resource;
use gerust::flow::Flow;

use hyper::header::{ContentLength, ContentType};
use hyper::server::{Http, Response, const_service, service_fn};

#[derive(Debug)]
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

fn main() {
    env_logger::init().unwrap();

    let addr = ([127, 0, 0, 1], 3000).into();

    let new_service = const_service(service_fn(|request| {
        let response = http::Response::default();

        let resource = DefaultResource { request: request, response: response };
        let mut flow = Flow::new(resource);
        flow.execute();
        let DefaultResource { request, response } = flow.finish();

        Ok(response)
    }));

    let mut server = Http::new().bind_compat(&addr, new_service).unwrap();
    server.no_proto();
    println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap();
}
