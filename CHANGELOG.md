# Changelog
All notable changes to this project will be documented in this file.
## [0.5.0] - 2020-04-26
### Added
- Added support for future 0.3 (async/await) #24 (https://github.com/filipdulic/bus-queue/pull/24)
  Implemented future 0.3 Sink for publisher and Stream for subscribers.
  Adds `crossbeam_channel` as a dependency for sending AtomicWakers to publisher.
- Added tests.
- Added a `missed_items_size` #25 (https://github.com/filipdulic/bus-queue/pull/25)
### Removed
- The sync module is removed (not needed any more). #24 (https://github.com/filipdulic/bus-queue/pull/24)
### Fixed
- Fixed bug in PartialEq. #24 (https://github.com/filipdulic/bus-queue/pull/24)
- Fixed bug in sub_count. #24 (https://github.com/filipdulic/bus-queue/pull/24)
## [0.4.1](https://github.com/filipdulic/bus-queue/pull/21) - 2019-07-30
### Added
- GetSubCount - Trait added to all Publisher struct.
## [0.4.0](https://github.com/filipdulic/bus-queue/pull/19) - 2019-07-13
### Added
- [Sync](https://doc.rust-lang.org/std/marker/trait.Sync.html) and [Send](https://doc.rust-lang.org/std/marker/trait.Send.html) Traits to all interfaces.
### Changed
- The **sync::Publisher** method **broadcast** now takes a mutable reference to self.
### Removed
- Interior mutability on the **Waker** struct.
