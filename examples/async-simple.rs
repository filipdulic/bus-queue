extern crate bus_queue;
extern crate futures;
extern crate tokio;

use bus_queue::async;
use futures::*;
use tokio::runtime::Runtime;

fn main() {
    let mut rt = Runtime::new().unwrap();
    let (tx, rx) = async::channel(4);

    let publisher = stream::iter_ok(vec![1, 2, 3, 4, 5])
        .forward(tx)
        .and_then(|(_, mut sink)| sink.close())
        .map_err(|_| ())
        .map(|_| ());

    rt.spawn(publisher);
    let collected = rt.block_on(rx.map(|x| *x).collect()).unwrap();
    assert_eq!(collected, vec![2, 3, 4, 5]);
}
