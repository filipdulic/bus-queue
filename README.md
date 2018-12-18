# bus-queue
Lock free bounded non blocking pub sub queue

The queue designed here is a publish subscribe pattern, where the publisher is never blocked by slow subscribers.
The side effect is that slow subscribers will miss messages. The intended use case are high throughput streams where receiving the latest message is prioritized over receiving the entire stream. Market Data Feeds, Live Streams, etc....

The underlying data-structure is a vector of Arc(s) providing efficient use of memory, with no copies. 
