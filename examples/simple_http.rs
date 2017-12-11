extern crate env_logger;

extern crate gerust;
extern crate mime;
extern crate futures;
extern crate http;

use gerust::resource::Resource;

use futures::sink::Sink;

#[derive(Debug, Default)]
struct OrderResource;

impl Resource for OrderResource {
    fn allowed_methods(&self) -> &'static [http::Method] {
        use http::method::Method;

        &[Method::GET, Method::HEAD, Method::PUT, Method::POST]
    }

    fn content_types_provided(&self) -> &'static [(mime::Mime, fn(&mut OrderResource, &mut gerust::flow::DelayedResponse))] {
        &[(mime::TEXT_HTML, OrderResource::to_html)]
    }

    fn content_types_accepted(&self) -> &'static [(mime::Mime, fn(&mut OrderResource, &mut gerust::flow::DelayedResponse))] {
        &[(mime::APPLICATION_JSON, OrderResource::from_json)]
    }
}

impl OrderResource {
    fn to_html(&mut self, resp: &mut gerust::flow::DelayedResponse) {
        use futures::Sink;

        resp.response_body().start_send(Ok("Hello, World!".into()));
    }

    fn from_json(&mut self, resp: &mut gerust::flow::DelayedResponse) {

    }
}

fn main() {
    env_logger::init().unwrap();
    // TBD Dispatching over multiple resources
    gerust::server::run_server::<OrderResource>(100)
}
