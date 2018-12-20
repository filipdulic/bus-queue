extern crate bus_queue;

use bus_queue::sync;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time;

fn main() {
    let (mut bus, rx1) = sync::channel(3);
    let rx2 = rx1.clone();
    let publisher = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(2000));
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
            match rx1.try_recv() {
                Err(e) => match e {
                    TryRecvError::Empty => (), //println!("b: Buffer empty"),
                    TryRecvError::Disconnected => {
                        println!("subscriber_1: Pub Disconnected!");
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
            match rx2.try_recv() {
                Err(e) => match e {
                    TryRecvError::Empty => (), //println!("c: Buffer empty"),
                    TryRecvError::Disconnected => {
                        println!("subscriber_2: Pub Disconnected!");
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
