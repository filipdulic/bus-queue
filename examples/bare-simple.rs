extern crate bus_queue;

use bus_queue::bare_channel;

fn main() {
    let (mut tx, rx) = bare_channel(10);
    vec![1, 2, 3, 4]
        .into_iter()
        .for_each(|x| tx.broadcast(x).unwrap());

    let received: Vec<i32> = rx.into_iter().map(|x| *x).collect();

    assert_eq!(vec![1, 2, 3, 4], received);
}
