// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

//! An Fleetspeak client connector library.
//!
//! This library exposes a set of functions for writing client-side Fleetspeak
//! services. Each of these functions operates on a global connection object
//! that is lazily established. If this global connection cannot be established,
//! the library will panic (because without this connection Fleetspeak will shut
//! the service down anyway).
//!
//! Note that each service should send startup information upon its inception
//! and continue to heartbeat from time to time to notify the Fleetspeak client
//! that it did not get stuck.

mod connection;
mod error;

use std::fs::File;
use std::sync::Mutex;
use std::time::Duration;

use lazy_static::lazy_static;

pub use self::connection::Packet;
pub use self::error::{ReadError, WriteError};

/// Sends a heartbeat signal to the Fleetspeak client.
///
/// All client services should heartbeat from time to time. Otherwise, from the
/// Fleetspeak perspective, the service is unresponsive and should be restarted.
///
/// The exact frequency of the required heartbeat is defined in the service
/// configuration file.
pub fn heartbeat() -> Result<(), WriteError> {
    locked(&CONNECTION.output, |buf| self::connection::heartbeat(buf))
}

/// Sends a system message with startup information to the Fleetspeak client.
///
/// All clients are required to send this information on startup. If the client
/// does not receive this information quickly enough, the service will be
/// killed.
///
/// The `version` string should contain a self-reported version of the service.
/// This data is used primarily for statistics.
pub fn startup(version: &str) -> Result<(), WriteError> {
    locked(&CONNECTION.output, |buf| self::connection::startup(buf, version))
}

/// Sends the message to the Fleetspeak server.
///
/// The message is delivered to the server-side service as specified by the
/// packet and optionally tagged with a type if specified. This optional message
/// type is irrelevant for Fleetspeak but might be useful for the service the
/// message is delivered to.
///
/// In case of any I/O failure or malformed message (e.g. due to encoding
/// problems), an error is reported.
///
/// # Examples
///
/// ```no_run
/// use fleetspeak::client::Packet;
///
/// fleetspeak::client::send(Packet {
///     service: String::from("example"),
///     kind: None,
///     data: String::from("Hello, World!"),
/// }).expect("failed to send the packet");
/// ```
pub fn send<M>(packet: Packet<M>) -> Result<(), WriteError>
where
    M: prost::Message,
{
    locked(&CONNECTION.output, |buf| self::connection::send(buf, packet))
}

/// Receives the message from the Fleetspeak server.
///
/// This function will block until there is a message to be read from the input.
/// Note that in particular it means your service will be unable to heartbeat
/// properly. If you are not expecting the message to arrive quickly, you should
/// use [`collect`] instead.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`collect`]: fn.collect.html
///
/// # Examples
///
/// ```no_run
/// match fleetspeak::client::receive::<String>() {
///     Ok(packet) => println!("Hello, {}!", packet.data),
///     Err(error) => eprintln!("failed to receive the packet: {}", error),
/// }
/// ```
pub fn receive<M>() -> Result<Packet<M>, ReadError>
where
    M: prost::Message + Default,
{
    locked(&CONNECTION.input, |buf| self::connection::receive(buf))
}

/// Collects the message from the Fleetspeak server.
///
/// Unlike [`receive`], `collect` will send heartbeat signals while polling for
/// the message at the specified `rate`.
///
/// This function is useful in the main loop of your service when it is not
/// supposed to do anything until a request from the server arrives. If your
/// service is actually awaiting for a specific message to come, you should
/// use [`receive`] instead.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`receive`]: fn.receive.html
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// match fleetspeak::client::collect::<String>(Duration::from_secs(1)) {
///     Ok(packet) => println!("Hello, {}!", packet.data),
///     Err(error) => eprintln!("failed to collected the packet: {}", error),
/// }
/// ```
pub fn collect<M>(rate: Duration) -> Result<Packet<M>, std::io::Error>
where
    M: prost::Message + Default + 'static,
{
    let (sender, receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let packet = receive();

        // We do not care whether the packet was sent, because this call can
        // fail only if the receiver closed itself. Since the receiver loops
        // indefinitely, it closes only in case there was a heartbeat error. But
        // at that point we are no longer interested in the packet.
        let _ = sender.send(packet);
    });

    loop {
        use std::sync::mpsc::TryRecvError::*;

        // The sender will block indefinitely until there is a message to pick,
        // so the disconnection branch is not possible.
        match receiver.try_recv() {
            Ok(packet) => return Ok(packet?),
            Err(Empty) => (),
            Err(Disconnected) => panic!(),
        }

        heartbeat()?;
        std::thread::sleep(rate);
    }
}

/// A connection to the Fleetspeak client.
///
/// The connection is realized through two files (specified by descriptors given
/// by the Fleetspeak client as environment variables): input and output. Each
/// of these files is guarded by a separate mutex to allow writing (e.g. for
/// sending heartbeat signals) when another thread might be busy with reading
/// messages.
struct Connection {
    input: Mutex<File>,
    output: Mutex<File>,
}

lazy_static! {
    static ref CONNECTION: Connection = {
        let mut input = open("FLEETSPEAK_COMMS_CHANNEL_INFD");
        let mut output = open("FLEETSPEAK_COMMS_CHANNEL_OUTFD");

        use self::connection::handshake;
        handshake(&mut input, &mut output).expect("handshake failure");

        Connection {
            input: Mutex::new(input),
            output: Mutex::new(output),
        }
    };
}

/// Executes the given function with a file extracted from the mutex.
///
/// It might happen that the mutex becomes poisoned and this call will panic in
/// result. This should not be a problem in practice, because mutex poisoning
/// is a result of one of the threads being aborted. In case of a such scenario,
/// it is likely the service needs to be restarted anyway.
fn locked<F, T, E>(mutex: &Mutex<File>, f: F) -> Result<T, E>
where
    F: FnOnce(&mut File) -> Result<T, E>
{
    let mut file = mutex.lock().expect("poisoned connection mutex");
    f(&mut file)
}

/// Opens a file object pointed by an environment variable.
///
/// Note that this function will panic if the environment variable `var` is not
/// a valid file descriptor (in which case the library cannot be initialized and
/// the service is unlikely to work anyway).
fn open(var: &str) -> File {
    let fd = std::env::var(var)
        .expect(&format!("invalid variable `{}`", var))
        .parse()
        .expect(&format!("failed to parse file descriptor"));

    // TODO: Add support for Windows.
    unsafe {
        std::os::unix::io::FromRawFd::from_raw_fd(fd)
    }
}
