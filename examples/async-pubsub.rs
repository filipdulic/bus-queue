extern crate bus_queue;
extern crate futures;
extern crate tokio_core;

use bus_queue::async::AsyncBus;
use futures::prelude::*;
use std::thread;
use std::time;
use tokio_core::reactor::Core;
fn main() {
    let mut async_bus: AsyncBus<u32> = AsyncBus::new(10);

    let mut streams = Vec::new();
    for _i in 0..10 {
        streams.push(async_bus.add_sub());
    }
    let mut vec = Vec::new();
    for (index, stream) in streams.into_iter().enumerate() {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::create(format!("fut_{}_log.txt", index)).unwrap();
        vec.push(thread::spawn(move || {
            let future = stream.for_each(move |curr| {
                writeln!(file, "{}\r", curr);
                file.flush().unwrap();
                futures::future::ok(())
            });
            Core::new().unwrap().run(future).unwrap();
        }));
    }
    let a = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(2000));
        let async_bus = &mut async_bus;
        for i in 0..50 {
            thread::sleep(time::Duration::from_millis(100));
            async_bus.send(i).wait().unwrap();
        }
    });
    a.join().unwrap();
    for t in vec {
        t.join().unwrap();
    }
}
