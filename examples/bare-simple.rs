extern crate bus_queue;

use bus_queue::bare_channel;

fn main() {
    let (mut tx,rx) = bare_channel(1);

    tx.broadcast(4).unwrap();
    assert_eq!(4,*rx.try_recv().unwrap());
}