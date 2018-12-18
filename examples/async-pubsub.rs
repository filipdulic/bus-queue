extern crate bus_queue;
extern crate futures;
extern crate tokio;

use bus_queue::async::AsyncBus;
use std::thread;
use std::time;
use thread::current;
use tokio::prelude::*;

fn main() {
    let mut async_bus: AsyncBus<u32> = AsyncBus::new(10);

    let mut streams = Vec::new();
    for _i in 0..10 {
        streams.push(async_bus.add_sub());
    }
    let mut vec = Vec::new();
    for stream in streams {
        vec.push(thread::spawn(move || {
            let future = stream.for_each(|curr| {
                println!("fut {:?} : {}", current().id(), curr);
                futures::future::ok(())
            });
            tokio::run(future);
        }));
    }
    let a = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(2000));
        for i in 0..40 {
            async_bus.push(i);
            thread::sleep(time::Duration::from_millis(500));
        }
    });

    a.join().unwrap();
    for th in vec {
        th.join().unwrap();
    }
}
