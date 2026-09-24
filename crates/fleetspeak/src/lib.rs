// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

//! A [Fleetspeak] client connector library.
//!
//! This library exposes a set of utilities for writing client-side Fleetspeak
//! services.
//!
//! Note that each service should send startup information upon its inception
//! and continue to heartbeat from time to time to notify the Fleetspeak client
//! that it did not get stuck.
//!
//! [Fleetspeak]: https://github.com/google/fleetspeak

mod io;

use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Communication channel with the Fleetspeak process.
pub struct Comms {
    // TODO(rust-lang/rust#134645): Migrate to `std::sync::nonpoison::Mutext`
    // (or `std::sync::ReentrantLock`) once stable.
    raw_out: Mutex<crate::io::CommsOutRaw>,
    raw_in: Mutex<crate::io::CommsInRaw>,
    last_heartbeat: Mutex<Option<Instant>>,
}

impl Comms {

    // Returns a [`Comms`] instance given by the parent Fleetspeak process.
    ///
    /// # Safety
    ///
    /// This function must be invoked where the environment is guaranteed not to
    /// have been tampered with (e.g. at the beginning of the `main` function),
    /// so that it contains values really set by Fleetspeak (and not e.g. file
    /// descriptors of Rust-allocated resources "leaked" by [`as_raw_fd`][1]).
    ///
    /// This is to adhere to [I/O safety][2] requirements.
    ///
    /// Synchronization is done on per-instance basis, so the users should
    /// ensure that this is called only once in the program lifetime.
    ///
    /// [1]: https://doc.rust-lang.org/std/os/fd/trait.AsRawFd.html#tymethod.as_raw_fd
    /// [2]: https://rust-lang.github.io/rfcs/3128-io-safety.html
    pub unsafe fn from_env() -> std::io::Result<Comms> {
        // SAFETY: Safety contract is the same as for the outer function.
        let mut raw_in = unsafe {
            crate::io::CommsInRaw::from_env()
        }.map_err(CommsInEnvError)?;

        // SAFETY: Safety contract is the same as for the outer function.
        let mut raw_out = unsafe {
            crate::io::CommsOutRaw::from_env()
        }.map_err(CommsOutEnvError)?;

        crate::io::handshake(&mut raw_in, &mut raw_out)
            .map_err(HandshakeError)?;

        Ok(Comms {
            raw_in: Mutex::new(raw_in),
            raw_out: Mutex::new(raw_out),
            last_heartbeat: Mutex::new(None),
        })
    }

    /// Sends a system message with startup information to the Fleetspeak
    /// client.
    ///
    /// All clients are required to send this information on startup. If the
    /// client does not receive this information quickly enough, the service
    /// will be killed.
    ///
    /// The `version` string should contain a self-reported version of the
    /// service. This data is used primarily for statistics.
    pub fn startup(&self, version: &str) -> std::io::Result<()> {
        let mut raw_out = self.raw_out.lock().unwrap();
        self::io::write_startup(&mut *raw_out, version)
    }

    /// Sends a heartbeat signal to the Fleetspeak client.
    ///
    /// All client services should heartbeat from time to time. Otherwise, from
    /// the Fleetspeak perspective, the service is unresponsive and should be
    /// restarted.
    ///
    /// The exact frequency of the required heartbeat is defined in the service
    /// configuration file.
    pub fn heartbeat(&self) -> std::io::Result<()> {
        let mut raw_out = self.raw_out.lock().unwrap();
        self::io::write_heartbeat(&mut *raw_out)
    }

    /// Sends a heartbeat signal to the Fleetspeak client but no more frequently
    /// than the specified `rate`.
    ///
    /// Note that the specified `rate` should be at least the rate defined in
    /// the Fleetspeak service configuration file. Because of potential slow-
    /// downs, some margin of error should be left.
    ///
    /// See documentation for the [`Comms::heartbeat`] method for more details.
    pub fn heartbeat_with_throttle(&self, rate: Duration) -> std::io::Result<()> {
        let mut last_heartbeat = self.last_heartbeat.lock().unwrap();

        match *last_heartbeat {
            Some(last_heartbeat) if last_heartbeat.elapsed() < rate => {
            // Do nothing if the last heartbeat happened more recently than the
            // specified heartbeat rate.
                return Ok(())
            }
            _ => (),
        }

        self.heartbeat()?;
        *last_heartbeat = Some(Instant::now());

        Ok(())
    }

    /// Sends the message to the Fleetspeak server.
    ///
    /// The data is delivered to the server-side service as specified by the
    /// message and optionally tagged with a type if specified. This optional
    /// message type is irrelevant for Fleetspeak but might be useful for the
    /// service the message is delivered to.
    ///
    /// In case of any I/O failure or malformed message (e.g. due to encoding
    /// problems), an error is reported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fleetspeak::Message;
    ///
    /// let comms = unsafe { fleetspeak::Comms::from_env() }.unwrap();
    ///
    /// comms.send(Message {
    ///     service: String::from("example"),
    ///     kind: None,
    ///     data: String::from("Hello, world!").into_bytes(),
    /// }).unwrap();
    /// ```
    pub fn send(&self, message: Message) -> std::io::Result<()> {
        let mut raw_out = self.raw_out.lock().unwrap();
        self::io::write_message(&mut *raw_out, message)
    }

    /// Creates an iterator over messages from the Fleetspeak server.
    ///
    /// The iterator will block until there is a message to be read from the
    /// input or the input reaches its end (in which case the iterator will
    /// end).
    ///
    /// Note that in particular it means your service will be unable to send
    /// heartbeat signals properly. If you are not expecting messages to arrive
    /// quickly, you should use [`Receiver::with_heartbeat`] method to adapt
    /// the iterator to one that does hearbeating in background.
    ///
    /// The iterator will yield `Err` values in case of any I/O failure or
    /// malformed message (e.g. due to parsing issues or when some expected
    /// fields were not present).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// let comms = unsafe { fleetspeak::Comms::from_env() }.unwrap();
    ///
    /// for message in comms.receiver()
    ///     .map(Result::unwrap)
    /// {
    ///     let name = std::str::from_utf8(&message.data)
    ///         .expect("invalid message content");
    /// }
    /// ```
    pub fn receiver(&self) -> Receiver<'_> {
        Receiver {
            comms: self,
        }
    }

    /// Attempts to receive a message from the Fleetspeak server.
    ///
    /// This function will block until there is a message to be read from the
    /// input or the input reaches its end (in which case the call will return
    /// `None`).
    ///
    /// Note that in particular it means your service will be unable to
    /// heartbeat properly. If you are not expecting the message to arrive
    /// quickly, you should use [`Comms::try_receive_with_heartbeat`] instead.
    ///
    /// In case of any I/O failure or malformed message (e.g. due to parsing
    /// issues or when some fields are not being present), an error is reported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let comms = unsafe { fleetspeak::Comms::from_env() }.unwrap();
    ///
    /// let message = comms.try_receive().unwrap()
    ///     .expect("no message");
    ///
    /// let name = std::str::from_utf8(&message.data)
    ///     .expect("invalid message content");
    ///
    /// println!("Hello, {name}!");
    /// ```
    pub fn try_receive(&self) -> std::io::Result<Option<Message>> {
        let mut raw_in = self.raw_in.lock().unwrap();
        self::io::try_read_message(&mut *raw_in)
    }

    /// Receive a message from the Fleetspeak server, sending hearbeat signals
    /// in background at the specified `rate`.
    ///
    /// This function is useful in the main loop of your service when it is not
    /// supposed to do anything until a request from the server arrives. If your
    /// service is actually awaiting for a specific message to come, you should
    /// use [`try_receive`] instead.
    ///
    /// In case of any I/O failure or malformed message (e.g. due to parsing
    /// issues or when some fields are not being present), an error is reported.
    ///
    /// [`try_receive`]: Comms::try_receive
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// let comms = unsafe { fleetspeak::Comms::from_env() }.unwrap();
    ///
    /// let message = comms.try_receive_with_heartbeat(Duration::from_secs(1)).unwrap()
    ///     .expect("no message");
    ///
    /// let name = std::str::from_utf8(&message.data)
    ///     .expect("invalid message content");
    ///
    /// println!("Hello, {name}!");
    /// ```
    pub fn try_receive_with_heartbeat(&self, rate: Duration) -> std::io::Result<Option<Message>> {
        std::thread::scope(|scope| {
            // TODO(rust-lang/rust#35121): Replace with `!` once stable.
            enum Never {
            }

            let (sender, receiver) = std::sync::mpsc::channel::<Never>();

            let thread = std::thread::Builder::new()
                // Our thread is pretty much dumb loops, so almost no stack size
                // is really necessary. We stick to 64 KiB as Rust runtime needs
                // some and to be on the safe side.
                .stack_size(64 * 1024)
                .spawn_scoped(scope, move || loop {
                    use std::sync::mpsc::RecvTimeoutError::*;

                    // We keep hearbeating until the sender disconnects (in
                    // which case the receiver will receive a disconnection
                    // error).
                    match receiver.recv_timeout(rate) {
                        Ok(never) => match never {},
                        Err(Disconnected) => return Ok(()),
                        Err(Timeout) => {
                            self.heartbeat()?;
                        }
                    }
                })?;

            let message = self.try_receive()?;

            // Notify the heartbeat thread to shut down. However, instead of
            // sending any real message we just shut the sender down and the
            // receiver will receive a disconnection error.
            drop(sender);

            match thread.join() {
                Ok(Ok(())) => (),
                Ok(Err(error)) => return Err(error),
                Err(error) => std::panic::resume_unwind(error),
            }

            Ok(message)
        })
    }
}

/// An iterator yielding messages from the Fleetspeak server.
///
/// It is created by the [`Comms::receiver`] method, see its docs for more
/// details.
pub struct Receiver<'comms> {
    comms: &'comms Comms,
}

impl<'comms> Receiver<'comms> {

    /// Creates a variant of this iterator that sends heartbeat signals at the
    /// specified `rate` in background.
    ///
    /// See documentation for the [`Comms::try_receive_with_heartbeat`] why it
    /// might be important.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// let comms = unsafe { fleetspeak::Comms::from_env() }.unwrap();
    ///
    /// for message in comms.receiver()
    ///     .with_heartbeat(std::time::Duration::from_secs(1))
    ///     .map(Result::unwrap)
    /// {
    ///     let name = std::str::from_utf8(&message.data)
    ///         .expect("invalid message content");
    /// }
    /// ```
    pub fn with_heartbeat(&self, rate: Duration) -> ReceiverWithHeartbeat<'comms> {
        ReceiverWithHeartbeat {
            comms: self.comms,
            rate,
        }
    }
}

impl<'comms> Iterator for Receiver<'comms> {
    type Item = std::io::Result<Message>;

    fn next(&mut self) -> Option<std::io::Result<Message>> {
        self.comms.try_receive().transpose()
    }
}

/// An iterator yielding messages from the Fleetspeak server, sending heartbeat
/// signals in background.
///
/// It is created by the [`Receiver::with_heartbeat`] method, see its docs for
/// more details.
pub struct ReceiverWithHeartbeat<'comms> {
    comms: &'comms Comms,
    rate: Duration,
}

impl<'comms> Iterator for ReceiverWithHeartbeat<'comms> {
    type Item = std::io::Result<Message>;

    fn next(&mut self) -> Option<std::io::Result<Message>> {
        self.comms.try_receive_with_heartbeat(self.rate).transpose()
    }
}

/// A Fleetspeak client communication message.
///
/// This structure represents incoming or outgoing message objects delivered by
/// Fleetspeak. This is a simplified version of the underlying Protocol Buffers
/// message that exposes too much irrelevant fields and makes the protocol easy
/// to misuse.
pub struct Message {
    /// A name of the server-side service that sent or should receive the data.
    pub service: String,
    /// An optional message type that can be used by the server-side service.
    pub kind: Option<String>,
    /// The data to sent to the specified service.
    pub data: Vec<u8>,
}

/// Sends a heartbeat signal to the Fleetspeak client.
///
/// All client services should heartbeat from time to time. Otherwise, from the
/// Fleetspeak perspective, the service is unresponsive and should be restarted.
///
/// The exact frequency of the required heartbeat is defined in the service
/// configuration file.
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn heartbeat() {
    COMMS.heartbeat()
        .expect("failed to send heartbeat")
}

/// Sends a heartbeat signal to the Fleetspeak client but no more frequently
/// than the specified `rate`.
///
/// Note that the specified `rate` should be at least the rate defined in the
/// Fleetspeak service configuration file. Because of potential slowdowns, some
/// margin of error should be left.
///
/// See documentation for the [`heartbeat`] function for more details.
///
/// [`heartbeat`]: crate::heartbeat
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn heartbeat_with_throttle(rate: Duration) {
    COMMS.heartbeat_with_throttle(rate)
        .expect("failed to send heartbeat")
}

/// Sends a system message with startup information to the Fleetspeak client.
///
/// All clients are required to send this information on startup. If the client
/// does not receive this information quickly enough, the service will be
/// killed.
///
/// The `version` string should contain a self-reported version of the service.
/// This data is used primarily for statistics.
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn startup(version: &str) {
    COMMS.startup(version)
        .expect("failed to send startup notification")
}

/// Sends the message to the Fleetspeak server.
///
/// The data is delivered to the server-side service as specified by the message
/// and optionally tagged with a type if specified. This optional message type
/// is irrelevant for Fleetspeak but might be useful for the service the message
/// is delivered to.
///
/// In case of any I/O failure or malformed message (e.g. due to encoding
/// problems), an error is reported.
///
/// # Examples
///
/// ```no_run
/// use fleetspeak::Message;
///
/// fleetspeak::send(Message {
///     service: String::from("example"),
///     kind: None,
///     data: String::from("Hello, world!").into_bytes(),
/// });
/// ```
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn send(message: Message) {
    COMMS.send(message)
        .expect("failed to send")
}

/// Receives a message from the Fleetspeak server.
///
/// This function will block until there is a message to be read from the input.
/// Note that in particular it means your service will be unable to heartbeat
/// properly. If you are not expecting the message to arrive quickly, you should
/// use [`receive_with_heartbeat`] instead.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`receive_with_heartbeat`]: crate::receive_with_heartbeat
///
/// # Examples
///
/// ```no_run
/// let message = fleetspeak::receive();
///
/// let name = std::str::from_utf8(&message.data)
///     .expect("invalid message content");
///
/// println!("Hello, {name}!");
/// ```
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn receive() -> Message {
    #[allow(deprecated)] // The outer function is deprecated as well.
    try_receive()
        .expect("end of input")
}

/// Attempts to receive a message from the Fleetspeak server.
///
/// This function will block until there is a message to be read from the input
/// or the input reaches its end (in which case it will return `None`).
///
/// Note that in particular it means your service will be unable to heartbeat
/// properly. If you are not expecting the message to arrive quickly, you should
/// use [`try_receive_with_heartbeat`] instead.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`try_receive_with_heartbeat`]: crate::try_receive_with_heartbeat
///
/// # Examples
///
/// ```no_run
/// match fleetspeak::try_receive() {
///     Some(message) => {
///         let name = std::str::from_utf8(&message.data)
///             .expect("invalid message content");
///
///         println!("Hello, {name}!");
///     }
///     None => {
///         println!("No more messages!")
///     }
/// }
/// ```
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn try_receive() -> Option<Message> {
    COMMS.try_receive()
        .expect("failed to receive")
}

/// Receive a message from the Fleetspeak server, heartbeating in background.
///
/// Unlike [`receive`], `collect` will send heartbeat signals at the specified
/// `rate` while waiting for the message.
///
/// This function is useful in the main loop of your service when it is not
/// supposed to do anything until a request from the server arrives. If your
/// service is actually awaiting for a specific message to come, you should
/// use [`receive`] instead.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`receive`]: crate::receive
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// let message = fleetspeak::receive_with_heartbeat(Duration::from_secs(1));
///
/// let name = std::str::from_utf8(&message.data)
///     .expect("invalid message content");
///
/// println!("Hello, {name}!");
/// ```
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn receive_with_heartbeat(rate: Duration) -> Message {
    #[allow(deprecated)] // The outer function is deprecated as well.
    try_receive_with_heartbeat(rate)
        .expect("end of input")
}

/// Receive a message from the Fleetspeak server, heartbeating in background.
///
/// Unlike [`try_receive`], `try_receive_with_heartbeat` will send heartbeat
/// signals at the specified `rate` while waiting for the message.
///
/// This function is useful in the main loop of your service when it is not
/// supposed to do anything until a request from the server arrives. If your
/// service is actually awaiting for a specific message to come, you should
/// use [`try_receive`] instead.
///
/// In case of the end of the input, `None` is returned.
///
/// In case of any I/O failure or malformed message (e.g. due to parsing issues
/// or when some fields are not being present), an error is reported.
///
/// [`try_receive`]: crate::try_receive
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// match fleetspeak::try_receive_with_heartbeat(Duration::from_secs(1)) {
///     Some(message) => {
///         let name = std::str::from_utf8(&message.data)
///             .expect("invalid message content");
///
///         println!("Hello, {name}!");
///     }
///     None => {
///         println!("No more messages!")
///     }
/// }
/// ```
#[deprecated(since = "0.4.5", note = "not fully safe, use `Comms`")]
pub fn try_receive_with_heartbeat(rate: Duration) -> Option<Message> {
    COMMS.try_receive_with_heartbeat(rate)
        .expect("failed to receive")
}

static COMMS: LazyLock<Comms> = LazyLock::new(|| {
    // SAFETY: This is not entirely safe (we cannot guarantee when this code
    // runs and what is inside the environment variables at that time).
    //
    // Unfortunately, it cannot be made entirely safe, this is why functions
    // using this are marked as deprecated and are slated for removal in the
    // next version.
    let comms = unsafe {
        Comms::from_env()
    }.expect("comms initialization failure");

    log::info!("comms initialized");

    comms
});

#[derive(Debug)]
struct HandshakeError(std::io::Error);

impl std::fmt::Display for HandshakeError {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fleetspeak handshake failure: {}", self.0)
    }
}

impl std::error::Error for HandshakeError {
}

impl From<HandshakeError> for std::io::Error {

    fn from(error: HandshakeError) -> std::io::Error {
        std::io::Error::other(error)
    }
}

#[derive(Debug)]
struct CommsInEnvError(crate::io::CommsEnvError);

impl std::fmt::Display for CommsInEnvError {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid Fleetspeak input pipe env var: {}", self.0)
    }
}

impl std::error::Error for CommsInEnvError {
}

impl From<CommsInEnvError> for std::io::Error {

    fn from(error: CommsInEnvError) -> std::io::Error {
        std::io::Error::other(error)
    }
}

#[derive(Debug)]
struct CommsOutEnvError(crate::io::CommsEnvError);

impl std::fmt::Display for CommsOutEnvError {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid Fleetspeak output pipe env var: {}", self.0)
    }
}

impl std::error::Error for CommsOutEnvError {
}

impl From<CommsOutEnvError> for std::io::Error {

    fn from(error: CommsOutEnvError) -> std::io::Error {
        std::io::Error::other(error)
    }
}
