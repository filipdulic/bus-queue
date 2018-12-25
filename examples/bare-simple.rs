extern crate bus_queue;

use bus_queue::bare_channel;

fn main() {
    let (mut tx, rx) = bare_channel(10);
    (1..15).for_each(|x| tx.broadcast(x).unwrap());

    let received: Vec<i32> = rx.into_iter().map(|x| *x).collect();
    // Test that only the last 10 elements are in the received list.
    let expected: Vec<i32> = (5..15).collect();

    assert_eq!(expected, received);
}
