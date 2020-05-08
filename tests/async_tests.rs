use bus_queue::bounded;
use futures::{executor, stream::StreamExt, task::SpawnExt, SinkExt};
use rand::Rng;
use std::time::Duration;

// pool.spawn alternative
// pool.spawn(stream.forward(publisher).map(drop)).unwrap();

#[test]
fn test_subscriber_item_drop_related_to_ratio_of_timing() {
    const LEAD_IN_TIME: Duration = Duration::from_millis(10);
    const MIN_PUB_MS: u64 = 2;
    const MAX_PUB_MS: u64 = 10;
    const MIN_SUB_MULTIPLIER: u64 = 2;
    const MAX_SUB_MULTIPLIER: u64 = 10;
    const NUMBER_OF_GENERATED: usize = 1000;
    let mut rng = rand::thread_rng();
    let pub_ms = rng.gen_range(MIN_PUB_MS, MAX_PUB_MS);
    let pub_time = Duration::from_millis(pub_ms);
    let sub_multiplier = rng.gen_range(MIN_SUB_MULTIPLIER, MAX_SUB_MULTIPLIER);
    let sub_time = Duration::from_millis(sub_multiplier * pub_ms);
    let pool = executor::ThreadPool::new().unwrap();
    let (mut publisher, mut subscriber) = bounded::<usize>(1);
    pool.spawn(async move {
        std::thread::sleep(LEAD_IN_TIME);
        for i in 0usize..NUMBER_OF_GENERATED {
            std::thread::sleep(pub_time);
            publisher.send(i).await.unwrap()
        }
    })
    .unwrap();
    let vec: Vec<usize> = executor::block_on(async move {
        let mut vec = Vec::new();
        loop {
            std::thread::sleep(sub_time);
            match subscriber.next().await {
                Some(item) => vec.push(*item),
                _ => return vec,
            }
        }
    });
    assert!(
        (vec.len() >= (NUMBER_OF_GENERATED / (sub_multiplier as usize + 1usize)))
            && (vec.len() <= (NUMBER_OF_GENERATED / (sub_multiplier as usize - 1usize)))
    )
}
