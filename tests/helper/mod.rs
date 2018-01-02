use http;
use futures::sync::oneshot;
use futures::Future;
use futures_cpupool::CpuPool;
use gerust::flow::{HttpFlow, Flow};
use gerust::Body;
use gerust::resource::Resource;
use std::fmt::Debug;

pub fn execute<R>(resource: R, req: http::Request<Body>) -> http::Response<Body>
    where R: Resource + Debug + Send {

    let pool = CpuPool::new(2);

    let (sx, rx): (_, _) = oneshot::channel::<http::Response<Body>>();

    let result_future = pool.spawn(rx);

    pool.spawn_fn(move || {
        let mut flow = HttpFlow::new();

        flow.execute(resource, req, sx);
        let res: Result<(),()> = Ok(());
        res
    }).wait().expect("Test harness: Internal error during flow execution");
   
    result_future.wait().expect("Test harness: Internal error in Response handling")
}