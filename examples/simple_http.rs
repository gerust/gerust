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

use gerust::resource::{Resource, Handles};

use futures::sink::Sink;
use futures::Stream;
use futures::Future;

#[derive(Debug, Default)]
struct OrderResource {
    order: Option<Order>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Order {
    id: String,
    title: String
}

impl Resource for OrderResource {
    fn post_is_create(&self) -> bool { true }

    fn create_path(&self) -> String {
        format!("orders/{}", self.order.as_ref().unwrap().id )
    }

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

    fn process_post(&self, response: &mut gerust::flow::DelayedResponse) -> bool {
        //println!("received order: {:?}", self.order);

        true
    }
}

impl Handles for OrderResource {
    type Item = Order;

    fn handle(&mut self, item: Self::Item, request: &mut http::Request<hyper::Body>, response: &mut gerust::flow::DelayedResponse) {
        self.order = Some(item);

        println!("received order: {:?}", self.order);
    }
}

pub trait JSON: Handles where <Self as gerust::resource::Handles>::Item: serde::de::DeserializeOwned {
    fn from_json(&mut self, request: &mut http::Request<hyper::Body>, response: &mut gerust::flow::DelayedResponse) {
        // remove wait() here
        let item = request.body_mut().concat2()
            .then(|body| {
                serde_json::from_slice::<<Self as Handles>::Item>(&body.unwrap())
            }).wait();

        self.handle(item.unwrap(), request, response)
    }
}

impl JSON for OrderResource {}

impl OrderResource {
    fn to_html(&mut self, resp: &mut gerust::flow::DelayedResponse) {
        use futures::Sink;

        resp.response_body().start_send(Ok("Hello, World!".into()));
    }
}

fn main() {
    env_logger::init().unwrap();
    // TBD Dispatching over multiple resources
    gerust::server::run_server::<OrderResource>(100)
}
