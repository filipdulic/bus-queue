extern crate bus_queue;

use bus_queue::Bus;
use std::thread;
use std::time;

fn main() {
    let bus: Bus<u32> = Bus::new(10);
    let mut rx1 = bus.add_sub();
    let mut rx2 = bus.add_sub();
    let a = thread::spawn(move || {
        let mut vec = Vec::new();
        for i in 0..40 {
            vec.push(i);
        }
        thread::sleep(time::Duration::from_millis(2000));
        for i in vec {
            bus.push(i);
            //bus.print();
            thread::sleep(time::Duration::from_millis(500));
        }
    });

    let b = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        for _i in 0..100 {
            match rx1.recv() {
                None => println!("b: empty;"),
                Some(ref arc_obj) => println!("b: {}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    let c = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        for _i in 0..100 {
            match rx2.recv() {
                None => println!("c: empty"),
                Some(ref arc_obj) => println!("c: {}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    });
    a.join().unwrap();
    b.join().unwrap();
    c.join().unwrap();
}
