# Changelog
All notable changes to this project will be documented in this file.
## 0.5.3 - 2020-05-10
### Added
- [Issue #36](https://github.com/filipdulic/bus-queue/issues/36) - [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Refactor Senders and Reciever to use an internal Channel which
                                                                                                                                         provides most of the logic.
- [Issue #44](https://github.com/filipdulic/bus-queue/issues/44) - [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45)  - Add SwapSlot Trait.
- [Issue #38](https://github.com/filipdulic/bus-queue/issues/38) - [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Add RwLock flavor.
- [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Refactor folder structure to have individual files for Channel,
                                                                        Sender, Reciever, Publisher and Subscriber.
- [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Integrate Channel with flavors using the SwapSlot Trait.
- [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Copy [piper::Event](https://github.com/stjepang/piper) into the project until event is exposed on
                                                                        crates.io by its author.
- [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Add Tarapaulin github workflow.
- [Pull Request #45](https://github.com/filipdulic/bus-queue/pull/45) - Code coverage increased to over 90%.
## 0.5.2 - 2020-05-09
### Fixed
- [Issue #29](https://github.com/filipdulic/bus-queue/issues/28) - [Pull Request #35](https://github.com/filipdulic/bus-queue/pull/35) - Index overflow not handled.
### Added
- [Pull Request #32](https://github.com/filipdulic/bus-queue/pull/32) Refactor wakers to use piper::Event to notify pending streams an item is ready.
- [Issue #40](https://github.com/filipdulic/bus-queue/issues/40) - [Pull Request #43](https://github.com/filipdulic/bus-queue/pull/43) - Add inital integration tests.
## 0.5.1 - 2020-05-03
### Fixed
- [Issue #28](https://github.com/filipdulic/bus-queue/issues/28) - [Pull Request #30](https://github.com/filipdulic/bus-queue/pull/30) Datarace where the reader reads the latest value in the queue insted of the oldest one.
## 0.5.0 - 2020-04-26
### Added
- [Pull Request #24](https://github.com/filipdulic/bus-queue/pull/24) - Added support for future 0.3 (async/await)
  Implemented future 0.3 Sink for publisher and Stream for subscribers.
  Adds `crossbeam_channel` as a dependency for sending AtomicWakers to publisher.
- [Pull Request #24](https://github.com/filipdulic/bus-queue/pull/24) - Added tests.
- [Pull Request #25](https://github.com/filipdulic/bus-queue/pull/25) - Added a `missed_items_size`
### Removed
- [Pull Request #24](https://github.com/filipdulic/bus-queue/pull/24) The sync module has been removed (not needed anymore).
### Fixed
- [Pull Request #24](https://github.com/filipdulic/bus-queue/pull/24) - Fixed bug in PartialEq.
- [Pull Request #24](https://github.com/filipdulic/bus-queue/pull/24) Fixed bug in sub_count.
## 0.4.1- 2019-07-30
### Added
- [Pull Request #21](https://github.com/filipdulic/bus-queue/pull/21) - GetSubCount - Trait added to all Publisher struct.
## 0.4.0 - 2019-07-13
### Added
- [Pull Request #19](https://github.com/filipdulic/bus-queue/pull/19) - [Sync](https://doc.rust-lang.org/std/marker/trait.Sync.html) and [Send](https://doc.rust-lang.org/std/marker/trait.Send.html) Traits to all interfaces.
### Changed
- The **sync::Publisher** method **broadcast** now takes a mutable reference to self.
### Removed
- Interior mutability on the **Waker** struct.
