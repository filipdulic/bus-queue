extern crate bus_queue;

use bus_queue::sync;
fn main() {
    // Create a sync channel
    let (tx, rx) = sync::channel(10);
    // spawn tx thread, broadcast all and drop publisher.
    let tx_t = std::thread::spawn(move || {
        (1..15).for_each(|x| tx.broadcast(x).unwrap());
    });
    // small sleep for the tx thread to send and close, before rx thread is called
    std::thread::sleep(std::time::Duration::from_millis(100));

    // spawn rx thread to get all the items left in the buffer
    let rx_t = std::thread::spawn(move || {
        let received: Vec<i32> = rx.map(|x| *x).collect();
        // Test that only the last 10 elements are in the received list.
        let expected: Vec<i32> = (5..15).collect();
        assert_eq!(received, expected);
    });

    tx_t.join().unwrap();
    rx_t.join().unwrap();
}
