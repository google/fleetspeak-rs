Fleetspeak (for Rust)
=====================

[![Travis CI build status][travis-status]][travis-builds]
[![Crate][crate-badge]][crate]
[![Documentation][docs-badge]][docs]

[Fleetspeak][fleetspeak] is a communication framework with a focus on security
monitoring. Currently, it is primarily used in the [GRR][grr] project (an remote
live forensics framework).

This repository contains a library for writing code in the [Rust][rust] language
for client-side Fleetspeak services. In a nutshell, this library is just a set
of functions for sending and receiving messages from the Fleetspeak client.

Currently there are no plans to provide capabilities for writing server-side
capabilities as well. Since server-side services communicate with the Fleetspeak
server through [gRPC][grpc], having a sufficiently ergonomic gRPC library should
be more than enough for such purposes.

This project is not an official Google product, is under heavy development and
should not be used for any production code. It is merely a proof of concept and
part of the experiment of rewriting the GRR client in Rust.

[fleetspeak]: https://github.com/google/fleetspeak
[grr]: https://github.com/google/grr
[rust]: https://rust-lang.org
[grpc]: https://grpc.io

[travis-builds]: https://travis-ci.org/google/fleetspeak-rs
[travis-status]: https://travis-ci.org/google/fleetspeak-rs.svg?branch=master
[crate]: https://crates.io/crates/fleetspeak
[crate-badge]: https://img.shields.io/crates/v/fleetspeak.svg
[docs]: https://docs.rs/fleetspeak
[docs-badge]: https://docs.rs/fleetspeak/badge.svg
