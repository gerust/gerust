extern crate futures_cpupool;
extern crate tokio_core;

use http;
use futures;
use hyper;

use std::marker::PhantomData;

use resource::Resource;
use flow::{Flow, HttpFlow};

use hyper::server::{Http};

use futures::sync::oneshot;
use futures::Future;

use std::fmt::Debug;

struct GerustService<'a, R> where R: Resource + Default + Debug {
    pool: &'a futures_cpupool::CpuPool,
    handle: tokio_core::reactor::Remote,
    resource: PhantomData<R>
}

impl<'a, R> hyper::server::Service for GerustService<'a, R>
    where R: Resource + Default + Debug
{
    type Request = http::Request<hyper::Body>;
    type Response = http::Response<hyper::Body>;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let (sx, rx): (futures::sync::oneshot::Sender<Self::Response>, _) = oneshot::channel::<Self::Response>();

        let f = futures::future::lazy(move || {
            let resource = R::default();
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

// TODO: Relax these bounds
pub fn run_server<R: Resource + Debug + Default>(threads: usize) {
    let addr = ([127, 0, 0, 1], 3000).into();

    let core = tokio_core::reactor::Core::new().unwrap();

    let pool = futures_cpupool::CpuPool::new(threads);

    let remote = core.remote();

    let service = move || {
        Ok(GerustService { pool: &pool, handle: remote.clone(), resource: PhantomData::<R> })
    };

    let server = Http::new().bind_compat(&addr, service).unwrap();
    //server.no_proto();
    info!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap();
}
