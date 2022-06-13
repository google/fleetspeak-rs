0.2.1 (2022-06-13)
==================

  * Vendored the Protocol Buffers standard library into the repository.
  * Pinned version of the `protobuf` crate to 2.8.2. This change was forced by
    internal requirements and should be treated only as a temporary measure.

0.2.0 (2022-06-06)
==================

  * Migrated from the [`prost`] crate to [`protobuf`] for Protocol Buffers
    support.
  * Change API nomenclature to be consistent with Fleetspeak libraries for other
    languages.

[`prost`]: https://crates.io/crates/prost
[`protobuf`]: https://crates.io/crates/protobuf

0.1.3 (2021-07-14)
==================

  * Added heartbeat variant that supports throttling.

0.1.2 (2020-05-16)
==================

  * Added support for Windows.

0.1.1 (2020-03-24)
==================

  * Fixed message collection logic to not leak threads.
  * Added basic logging for interesting events.

0.1.0 (2020-03-22)
==================

Initial version of the library.
