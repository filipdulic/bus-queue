# Changelog
All notable changes to this project will be documented in this file.
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
