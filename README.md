# bus-queue
##Lock-free Bounded Non-Blocking Pub-Sub Queue

The queue designed here is a publish subscribe pattern, where the publisher is never blocked by slow subscribers.
The side effect is that slow subscribers will miss messages. The intended use case are high throughput streams where receiving the latest message is prioritized over receiving the entire stream. Market Data Feeds, Live Streams, etc....

The underlying data-structure is a vector of Arc(s) providing efficient use of memory, with no copies.

##Features
* Lock-Free Write/Read - Lock-Free for Publisher and Lock-Free for Subscribers.
* Bounded - Constant size of memory used, max is `sizeof(MsgObject)* (queue_size + sub_cnt + 1)`...
...This is the edge case where each subscriber is holding a ref to an object while the publisher has published a full length of queue in the mean time.
* Non-Blocking - The queue never blocks the publisher, slow subscribers miss data proportinal to their speed.
* Pub-Sub - Every Subscriber that can keep up with the Publisher will recieve all the data the Publisher publishes.
* Sync/Async - both interfaces are provided and can be mixed. For example you can publish with a sync Publisher, an recive with an async Subscriber, and vice-versa.
