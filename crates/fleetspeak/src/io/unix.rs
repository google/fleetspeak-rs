// Copyright 2024 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use super::{CommsEnvError, CommsEnvErrorRepr};

/// Alternative for [`std::io::Stdin`] for communicating with Fleetspeak.
///
/// Reading from this communication channel is not synchronized nor buffered.
pub struct CommsInRaw {
    /// File descriptor of the input channel passeed by the Fleetspeak process.
    fd: libc::c_int,
}

/// Alternative for [`std::io::Stdout`] for communicating with Fleetspeak.
///
/// Writing to this communication channel is not synchronized nor buffered.
pub struct CommsOutRaw {
    /// File descriptor of the output channel passeed by the Fleetspeak process.
    fd: libc::c_int,
}

impl CommsInRaw {

    /// Returns a [`CommsIn`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsInRaw, CommsEnvError> {
        Ok(CommsInRaw {
            fd: env_var_fd("FLEETSPEAK_COMMS_CHANNEL_INFD")?,
        })
    }
}

impl CommsOutRaw {

    /// Returns a [`CommsOut`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsOutRaw, CommsEnvError> {
        Ok(CommsOutRaw {
            fd: env_var_fd("FLEETSPEAK_COMMS_CHANNEL_OUTFD")?,
        })
    }
}

impl std::io::Read for CommsInRaw {

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // SAFETY: We do not have any assumptions on `self.fd`. We usually want
        // it to be a valid file descriptor but since it is passed to us from
        // the parent process, we cannot guarantee that it actually is.
        //
        // However, there is no unsafety here: in case we are not allowed to do
        // a read operation on this supposed descriptor, it will simply fail
        // (e.g. with `EBADF` if this is not actually a descriptor).
        //
        // The rest is just a function call as described in the docs [1, 2]: we
        // pass a valid buffer and the number of bytes that we want to read
        // (which is equal to the length of the buffer). We verify the result
        // afterwards.
        //
        // [1]: https://man7.org/linux/man-pages/man2/read.2.html
        // [2]: https://pubs.opengroup.org/onlinepubs/009604599/functions/read.html
        let count = unsafe {
            libc::read(self.fd, buf.as_mut_ptr().cast(), buf.len())
        };

        if count < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(count as usize)
    }
}

impl std::io::Write for CommsOutRaw {

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // SAFETY: We do not have any assumptions on `self.fd`. We usually want
        // it to be a valid file descriptor but since it is passed to us from
        // the parent process, we cannot guarantee that it actually is.
        //
        // However, there is no unsafety here: in case we are not allowed to do
        // a write operation on this supposed descriptor, it will simply fail
        // (e.g. with `EBADF` if this is not actually a descriptor).
        //
        // The rest is just a function call as described in the docs [1, 2]: we
        // pass a valid buffer and the number of bytes that we want to write
        // (which is equal to the length of the buffer). We verify the result
        // afterwards.
        //
        // [1]: https://man7.org/linux/man-pages/man2/write.2.html
        // [2]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/write.html
        let count = unsafe {
            libc::write(self.fd, buf.as_ptr().cast(), buf.len())
        };

        if count < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(count as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // We use `libc::write` for writing data which is not buffered, there
        // is nothing to flush.
        Ok(())
    }
}

/// Retrieves a file descriptor specified in the given environment variable.
fn env_var_fd<K>(key: K) -> Result<libc::c_int, CommsEnvError>
where
    K: AsRef<std::ffi::OsStr>,
{
    match std::env::var(key) {
        Ok(fd) => match fd.parse::<libc::c_int>() {
            Ok(fd) => Ok(fd),
            Err(_) => Err(CommsEnvError {
                repr: CommsEnvErrorRepr::NotParsable(fd.into()),
            }),
        }
        Err(std::env::VarError::NotPresent) => Err(CommsEnvError {
            repr: CommsEnvErrorRepr::NotSpecified,
        }),
        Err(std::env::VarError::NotUnicode(value)) => Err(CommsEnvError {
            repr: CommsEnvErrorRepr::NotParsable(value),
        }),
    }
}
