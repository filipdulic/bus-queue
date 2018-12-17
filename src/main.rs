extern crate arc_swap;

use arc_swap::ArcSwap;
use std::default::Default;
use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

pub struct BusReader<T: Display + Default> {
    buffer: Arc<Box<[ArcSwap<T>]>>,
    wi: Arc<AtomicUsize>,
    ri: usize,
    size: usize,
}
impl<T: Display + Default> BusReader<T> {
    pub fn recv(&mut self) -> Option<Arc<T>> {
        if self.ri == self.wi.load(Ordering::Relaxed) {
            return None;
        }
        let mut object: Arc<T>;
        loop {
            object = self.buffer[self.ri % self.size].load();
            if self.wi.load(Ordering::Relaxed) > self.ri + self.size {
                self.ri = self.wi.load(Ordering::Relaxed) - self.size;
            } else {
                self.ri += 1;
                return Some(object);
            }
        }
    }
}
pub struct Bus<T: Display + Default> {
    // atp to an array of atps of option<arc<t>>
    buffer: Arc<Box<[ArcSwap<T>]>>,
    wi: Arc<AtomicUsize>,
    size: usize,
}

impl<T: Display + Default> Bus<T> {
    pub fn new(size: usize) -> Self {
        let mut temp: Vec<ArcSwap<T>> = Vec::new();
        for _i in 0..size {
            temp.push(ArcSwap::from(Arc::new(T::default())));
        }

        Self {
            buffer: Arc::new(temp.into_boxed_slice()),
            wi: Arc::new(AtomicUsize::new(0)),
            size: size,
        }
    }
    pub fn add_sub(&self) -> BusReader<T> {
        BusReader {
            buffer: self.buffer.clone(),
            wi: self.wi.clone(),
            ri: 0,
            size: self.size,
        }
    }
    pub fn push(&self, object: T) {
        self.buffer[self.wi.load(Ordering::Relaxed) % self.size].store(Arc::new(object));
        self.wi.fetch_add(1, Ordering::Relaxed);
    }
    pub fn print(&self) {
        let temp = &self.buffer;
        println!("******print********{}", temp.len());
        for (index, object) in temp.into_iter().enumerate() {
            let me = object.load();
            println!("{} : Some({})", index, me);
        }
        println!("******print********");
    }
}

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
            bus.print();
            thread::sleep(time::Duration::from_millis(500));
        }
    });

    let b = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        for _i in 0..100 {
            match rx1.recv() {
                None => (), //println!("b: Got none weird!"),
                Some(ref arc_obj) => println!("b: {}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(100));
        }
    });

    let c = thread::spawn(move || {
        thread::sleep(time::Duration::from_millis(1000));
        for _i in 0..100 {
            match rx2.recv() {
                None => (), //println!("c: Got none weird!"),
                Some(ref arc_obj) => println!("c: {}", arc_obj),
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    });
    a.join().unwrap();
    b.join().unwrap();
    c.join().unwrap();
}
