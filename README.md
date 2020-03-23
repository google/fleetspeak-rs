Fleetspeak (for Rust)
=====================

[![Travis CI build status][travis-badge]][travis]
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

[travis]: https://travis-ci.org/google/fleetspeak-rs
[travis-badge]: https://travis-ci.org/google/fleetspeak-rs.svg?branch=master
[crate]: https://crates.io/crates/fleetspeak
[crate-badge]: https://img.shields.io/crates/v/fleetspeak.svg
[docs]: https://docs.rs/fleetspeak
[docs-badge]: https://docs.rs/fleetspeak/badge.svg

Using
-----

To write your service, first add this library to dependencies in your project's
`Cargo.toml` file:

```toml
[dependencies]
fleetspeak = "0.1.0"
```

Now, in your project, you can use functions such as `fleetspeak::send` and
`fleetspeak::receive` to communicate with the Fleetspeak client. Consult the
[documentation](https://docs.rs/fleetspeak) about the details. You can also
checkout the [example](examples/hello.rs).

Read the Fleetspeak manual to learn how to make the Fleetspeak client aware of
your service and launch it as a daemon.
