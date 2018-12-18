extern crate tokio;
extern crate bus_queue;

use bus_queue::async::Test;
use tokio::prelude::*;

fn main(){
    let test = Test::new(); // half a second
    let future = test.for_each(|curr| {
        println!("Counter: {}", curr);
        futures::future::ok(())
    });

    tokio::run(future)
}