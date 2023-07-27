// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

//! A [Fleetspeak] client connector library.
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
//!
//! [Fleetspeak]: https://github.com/google/fleetspeak

mod io;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;

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
pub fn heartbeat() {
    execute(&CONNECTION.output, |buf| self::io::write_heartbeat(buf))
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
pub fn heartbeat_with_throttle(rate: Duration) {
    lazy_static! {
        static ref LAST_HEARTBEAT: Mutex<Option<Instant>> = Mutex::new(None);
    }

    let mut last_heartbeat = LAST_HEARTBEAT.lock()
        .expect("poisoned heartbeat mutex");

    match *last_heartbeat {
        Some(last_heartbeat) if last_heartbeat.elapsed() < rate => {
            // Do nothing if the last heartbeat happened more recently than the
            // specified heartbeat rate.
            return;
        }
        _ => (),
    }

    heartbeat();
    *last_heartbeat = Some(Instant::now());
}

/// Sends a system message with startup information to the Fleetspeak client.
///
/// All clients are required to send this information on startup. If the client
/// does not receive this information quickly enough, the service will be
/// killed.
///
/// The `version` string should contain a self-reported version of the service.
/// This data is used primarily for statistics.
pub fn startup(version: &str) {
    execute(&CONNECTION.output, |buf| self::io::write_startup(buf, version))
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
pub fn send(message: Message) {
    execute(&CONNECTION.output, |buf| self::io::write_message(buf, message))
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
pub fn receive() -> Message {
    execute(&CONNECTION.input, |buf| self::io::read_message(buf))
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
pub fn receive_with_heartbeat(rate: Duration) -> Message {
    // TODO(rust-lang/rust#35121): Replace with `!` once stable.
    enum Never {
    }

    let (sender, receiver) = std::sync::mpsc::channel::<Never>();

    std::thread::spawn(move || {
        loop {
            use std::sync::mpsc::TryRecvError::*;

            // We keep hearbeating until the sender disconnects (in which case
            // the receiver will receive a disconnection error).
            match receiver.try_recv() {
                Ok(never) => match never {},
                Err(Empty) => (),
                Err(Disconnected) => return,
            }

            heartbeat();
            std::thread::sleep(rate);
        }
    });

    let message = receive();

    // Notify the heartbeat thread to shut down. However, instead of sending any
    // real message we just shut the sender down and the receiver will receive
    // a disconnection error.
    drop(sender);

    message
}

/// A connection to the Fleetspeak client.
///
/// The connection is realized through two files (specified by descriptors given
/// by the Fleetspeak client as environment variables): input and output. Each
/// of these files is guarded by a separate mutex to allow writing (e.g. for
/// sending heartbeat signals) when another thread might be busy with reading
/// messages.
struct Connection {
    input: Mutex<&'static mut std::fs::File>,
    output: Mutex<&'static mut std::fs::File>,
}

lazy_static! {
    static ref CONNECTION: Connection = {
        let mut input = file_from_env_var("FLEETSPEAK_COMMS_CHANNEL_INFD");
        let mut output = file_from_env_var("FLEETSPEAK_COMMS_CHANNEL_OUTFD");

        crate::io::handshake(&mut input, &mut output)
            .expect("handshake failure");

        log::info!(target: "fleetspeak", "handshake successful");

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
///
/// Any I/O error returned by the executed function indicates a fatal connection
/// failure and ends with a panic.
fn execute<F, T>(mutex: &Mutex<&'static mut std::fs::File>, f: F) -> T
where
    F: FnOnce(&mut std::fs::File) -> std::io::Result<T>,
{
    let mut file = mutex.lock().expect("poisoned connection mutex");
    match f(&mut file) {
        Ok(value) => value,
        Err(error) => panic!("connection failure: {}", error),
    }
}

/// Creates a [`File`] object specified in the given environment variable.
///
/// Note that this function will panic if the environment variable `var` is not
/// a valid file descriptor (in which case the library cannot be initialized and
/// the service is unlikely to work anyway).
///
/// This function returns a static mutable reference to ensure that the file is
/// never dropped.
///
/// [`File`]: std::fs::File
fn file_from_env_var(var: &str) -> &'static mut std::fs::File {
    let fd = std::env::var(var)
        .expect(&format!("invalid variable `{}`", var))
        .parse()
        .expect(&format!("failed to parse file descriptor"));

    // We run inside a critical section to guarantee that between verifying the
    // descriptor and using `std::File::from_raw_*` functions (see the comments
    // below) nothing closes it inbetween.
    let mutex = Mutex::new(());
    let guard = mutex.lock()
        .unwrap();

    // SAFETY: `std::fs::File::from_raw_fd` requires the file descriptor to be
    // valid and open. We verify this through `fcntl` and panic if the check
    // fails. Because we run within a critical section we are sure that the file
    // was not closed at the moment we wrap the raw descriptor.
    //
    // Note that the whole issue is more subtle than this. While we uphold the
    // safety requirements of `from_raw_fd`, we cannot guarantee that we are
    // exclusive owner of the descriptor or that the descriptor remains open
    // throughout the entirety of the process lifetime which might lead to other
    // kinds of undefined behaviour. As an additional safety measure we return
    // a static mutable reference to ensure that the file destructor is never
    // called. See the discussion in [1].
    //
    // [1]: https://github.com/rust-lang/unsafe-code-guidelines/issues/434
    #[cfg(target_family = "unix")]
    let file = unsafe {
        if libc::fcntl(fd, libc::F_GETFD) == -1 {
            let error = std::io::Error::last_os_error();
            panic!("invalid file descriptor '{fd}': {error}");
        }

        std::os::unix::io::FromRawFd::from_raw_fd(fd)
    };

    // SAFETY: `std::fs::File::from_raw_handle` requires the file handle to be
    // valid, open and closable via `CloseHandle`. We verify this through a call
    // to `GetFileType`: if the call fails or returns an unexpected file type,
    // we panic. We expect the type to be a named pipe: this is what Fleetspeak
    // should pass as and it is closable via `CloseHandle` [1] as required. We
    // run within a critical section we are sure that the file was not closed at
    // the moment we wrap the raw handle.
    //
    // See also remarks in the comment for the Unix branch of this method.
    //
    // [1]: https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle#remarks
    #[cfg(target_family = "windows")]
    let file = unsafe {
        use windows_sys::Win32::{
            Foundation::*,
            Storage::FileSystem::*,
        };

        // We use `identity` to specify the type for the `parse` call above.
        let handle = std::convert::identity::<HANDLE>(fd);

        // We fail both in case there is something wrong with the handle (in
        // which case the call to `GetFileType` should return unknown file type
        // and set the last error value) or the file type is not as expected.
        let file_type = GetFileType(handle);
        if file_type != FILE_TYPE_PIPE {
            let code = GetLastError();
            if code != NO_ERROR {
                let error = std::io::Error::from_raw_os_error(code as i32);
                panic!("invalid file descriptor '{handle}': {error}");
            } else {
                panic!("wrong type of file descriptor '{handle}': {file_type}");
            }
        }

        // `HANDLE` type from `windows-sys` and from the standard library are
        // different (one is a pointer and one is `isize`), so we have to cast
        // between them.
        let handle = handle as std::os::windows::raw::HANDLE;

        std::os::windows::io::FromRawHandle::from_raw_handle(handle)
    };

    drop(guard);

    Box::leak(Box::new(file))
}
