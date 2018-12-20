#![allow(non_snake_case)]
extern crate bus_queue;

use bus_queue::sync;
use std::thread;
use std::time;

fn main() {
    let (mut bus, rx1) = sync::channel(3);
    let rx2 = rx1.clone();
    let publisher = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(500));
        for i in 0..15 {
            match bus.broadcast(i.clone()) {
                Ok(_) => println!("publisher\t-->\t{}", i.clone()),
                Err(e) => println!("publisher error: {:?}", e),
            }
            thread::sleep(time::Duration::from_millis(500));
        }
    });

    let subscriber_1 = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        loop {
            match rx1.recv() {
                Err(e) => match e {
                    RecvError => {
                        println!("subscriber_1: {:?}", RecvError);
                        return ();
                    }
                },
                Ok(ref arc_obj) => println!("subscriber_1\t<--\t{}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    let subscriber_2 = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        loop {
            match rx2.recv() {
                Err(e) => match e {
                    RecvError => {
                        println!("subscriber_2: {:?}", RecvError);
                        return ();
                    }
                },
                Ok(ref arc_obj) => println!("subscriber_2\t<--\t{}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    });
    publisher.join().unwrap();
    subscriber_1.join().unwrap();
    subscriber_2.join().unwrap();
}
