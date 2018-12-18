extern crate bus_queue;
extern crate tokio;

use bus_queue::async::AsyncBus;
use std::thread;
use std::time;
use tokio::prelude::*;

fn main() {
    let mut async_bus: AsyncBus<u32> = AsyncBus::new(10);

    let stream = async_bus.add_sub();
    let future = stream.for_each(|curr| {
        println!("Counter: {}", curr);
        futures::future::ok(())
    });
    let a = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(2000));
        for i in 0..40 {
            async_bus.push(i);
            thread::sleep(time::Duration::from_millis(500));
        }
    });

    tokio::run(future);
    a.join().unwrap();
}
