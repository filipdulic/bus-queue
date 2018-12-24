extern crate bus_queue;
extern crate futures;
extern crate tokio;

use bus_queue::async;
use futures::future::Future;
use futures::*;
use tokio::runtime::Runtime;

fn subscriber(rx: async::Subscriber<i32>) -> impl Future<Item = (), Error = ()> {
    assert_eq!(
        rx.map(|x| *x).collect().wait().unwrap(),
        vec![1, 2, 3, 4, 5]
    );
    future::ok(())
}

fn main() {
    let mut rt = Runtime::new().unwrap();
    let (tx, rx): (async::Publisher<i32>, async::Subscriber<i32>) = async::channel(10);

    let publisher = stream::iter_ok(vec![1, 2, 3, 3, 5])
        .forward(tx)
        .and_then(|(_, mut sink)| sink.close())
        .map_err(|_| ())
        .map(|_| ());

    rt.spawn(publisher);
    rt.block_on(subscriber(rx)).unwrap();
}
