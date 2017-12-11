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

#[derive(Debug)]
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

struct GerustService<'a> {
    pool: &'a futures_cpupool::CpuPool,
    handle: tokio_core::reactor::Remote,
}

impl<'a> hyper::server::Service for GerustService<'a>
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

        self.handle.spawn(move |_handle| { thread } );

        // TODO: don't unwrap the response builder result here
        Box::from(rx.or_else(|_| Ok(http::response::Builder::new()
                .status(501).body(b"<h1>Internal Server Error</h1>".as_ref().into()).unwrap())))
    }
}

fn main() {
    env_logger::init().unwrap();

    let addr = ([127, 0, 0, 1], 3000).into();

    let core = tokio_core::reactor::Core::new().unwrap();

    let pool = futures_cpupool::CpuPool::new(100);

    let remote = core.remote();

    let service = move || {
        Ok(GerustService { pool: &pool, handle: remote.clone() })
    };

    let server = Http::new().bind_compat(&addr, service).unwrap();
    //server.no_proto();
    info!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap();
}
