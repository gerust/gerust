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

use hyper::header::{ContentLength, ContentType};
use hyper::server::{Http, const_service, service_fn};

use futures::sync::oneshot;
use futures::Future;

#[derive(Debug)]
struct DefaultResource;

impl Resource for DefaultResource {
//
//    fn response(&self) -> &Self::Response {
//        &self.response
//    }
//
//    fn response_mut(&mut self) -> &mut Self::Response {
//        &mut self.response
//    }

    fn content_types_allowed(&self) -> &'static [(mime::Mime, fn(&Self) -> ())] {
        &[(mime::TEXT_HTML, default_html)]
    }
}

fn default_html(resource: &DefaultResource) -> () {

}

struct GerustService<'a> {
    pool: &'a futures_cpupool::CpuPool,
    handle: tokio_core::reactor::Remote,
   // body: std::marker::PhantomData<&'a B>
}

impl<'a> hyper::server::Service for GerustService<'a>
    //where B: futures::Stream<Item = hyper::Chunk, Error = http::Error> + 'static
{
    type Request = http::Request<hyper::Body>;
    type Response = http::Response<hyper::Body>;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let (sx, rx): (futures::sync::oneshot::Sender<Self::Response>, _) = oneshot::channel::<Self::Response>();

        let f = futures::future::lazy(move || {
            let resource = DefaultResource;
            let mut flow = HttpFlow::new();

            flow.execute(resource, req, sx);
            futures::future::ok::<(), ()>(())
        });

        let thread = self.pool.spawn(f);

        self.handle.spawn(move |handle| {thread } );

        // TODO: don't unwrap the response builder result here
        Box::from(rx.or_else(|e| Ok(http::response::Builder::new()
                .status(501).body("<h1>Internal Server Error</h1>".as_bytes().into()).unwrap())))
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = ([127, 0, 0, 1], 3000).into();

    let core = tokio_core::reactor::Core::new().unwrap();

    let pool = futures_cpupool::CpuPool::new(4);

    let remote = core.remote();

    let service = move || {
        Ok(GerustService { pool: &pool, handle: remote.clone() })
    };


    let mut server = Http::new().bind_compat(&addr, service).unwrap();
    //server.no_proto();
    println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap();

//    core.run(server);
}
