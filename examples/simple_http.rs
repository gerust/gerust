#[macro_use]
extern crate log;
extern crate env_logger;

extern crate simple_server;

extern crate gerust;
extern crate http;
extern crate mime;

use gerust::resource::Resource;
use gerust::flow::Flow;

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

use simple_server::Server;

fn main() {
    env_logger::init().unwrap();

    let host = "127.0.0.1";
    let port = "7878";

    let server = Server::new(|request, mut response| {
        let body = response.body("test".as_bytes()).unwrap();

        let resource = DefaultResource { request: request, response: body };
        let mut flow = Flow::new(resource);
        flow.execute();

        let DefaultResource { request, response } = flow.finish();

        Ok(response)
    });

    server.listen(host, port);
}