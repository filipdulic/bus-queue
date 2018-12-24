extern crate bus_queue;

use bus_queue::sync;
use std::thread;
fn main() {
    // Create a sync channel
    let (mut tx, rx) = sync::channel(1);
    let t = thread::spawn(move || {
        let received = rx.recv().unwrap();
        assert_eq!(*received, 10);
    });
    tx.broadcast(10).unwrap();
    t.join().unwrap();
}
