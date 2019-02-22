extern crate bus_queue;
extern crate futures;
extern crate tokio;

use bus_queue::async_;
use futures::*;
use tokio::runtime::Runtime;

fn main() {
    let mut rt = Runtime::new().unwrap();
    let (tx, rx) = async_::channel(10);
    let sent: Vec<i32> = (1..15).collect();
    let publisher = stream::iter_ok(sent)
        .forward(tx)
        .and_then(|(_, mut sink)| sink.close())
        .map_err(|_| ())
        .map(|_| ());

    rt.spawn(publisher);

    let received: Vec<i32> = rt.block_on(rx.map(|x| *x).collect()).unwrap();
    // Test that only the last 10 elements are in the received list.
    let expected: Vec<i32> = (5..15).collect();
    assert_eq!(expected, received);
}
