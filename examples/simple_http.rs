#[macro_use]
extern crate log;
extern crate env_logger;

extern crate hyper;

extern crate gerust;
extern crate http;
extern crate mime;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;

use gerust::resource::Resource;
use gerust::flow::{Flow, HttpFlow};

use hyper::server::{Http};

use futures::sync::oneshot;
use futures::Future;

#[derive(Debug, Default)]
struct DefaultResource;

impl Resource for DefaultResource {
    fn content_types_allowed(&self) -> &'static [(mime::Mime, fn(&mut DefaultResource, &mut gerust::flow::DelayedResponse))] {
        &[(mime::TEXT_HTML, DefaultResource::html)]
    }
}

impl DefaultResource {
    fn html(&mut self, resp: &mut gerust::flow::DelayedResponse) {
        use futures::Sink;

        resp.response_body().start_send(Ok("Hello, World!".into()));
    }
}

fn main() {
    env_logger::init();
    gerust::server::run_server::<DefaultResource>(100)
}
