extern crate env_logger;

extern crate gerust;
extern crate mime;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use gerust::resource::Resource;

use futures::sink::Sink;
use futures::Stream;
use futures::Future;

#[derive(Debug, Default)]
struct OrderResource;

#[derive(Debug, Deserialize, Serialize)]
struct Order {
    id: String,
    title: String
}

impl Resource for OrderResource {
    fn allowed_methods(&self) -> &'static [http::Method] {
        use http::method::Method;

        &[Method::GET, Method::HEAD, Method::PUT, Method::POST]
    }

    fn content_types_provided(&self) -> &'static [(mime::Mime, fn(&mut OrderResource, &mut gerust::flow::DelayedResponse))] {
        &[(mime::TEXT_HTML, OrderResource::to_html)]
    }

    fn content_types_accepted(&self) -> &'static [(mime::Mime, fn (&mut Self, request: &mut http::Request<hyper::Body>, response: &mut gerust::flow::DelayedResponse) -> ())] {
        &[(mime::APPLICATION_JSON, OrderResource::from_json)]
    }
}

impl OrderResource {
    fn to_html(&mut self, resp: &mut gerust::flow::DelayedResponse) {
        use futures::Sink;

        resp.response_body().start_send(Ok("Hello, World!".into()));
    }

    fn from_json(&mut self, request: &mut http::Request<hyper::Body>, response: &mut gerust::flow::DelayedResponse) {
        request.body_mut().concat2()
            .and_then(|body| {
                let order = serde_json::from_slice::<Order>(&body);
                println!("received order: {:?}", order);

                Ok(())
            }).wait();
    }
}

fn main() {
    env_logger::init().unwrap();
    // TBD Dispatching over multiple resources
    gerust::server::run_server::<OrderResource>(100)
}
