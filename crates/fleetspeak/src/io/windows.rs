// Copyright 2024 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use super::{CommsEnvError, CommsEnvErrorRepr};

/// Alternative for [`std::io::Stdin`] for communicating with Fleetspeak.
///
/// Reading from this communication channel is not synchronized nor buffered.
pub struct CommsInRaw {
    /// File handle of the input channel passed by the Fleetspeak process.
    handle: windows_sys::Win32::Foundation::HANDLE,
}

/// Alternative for [`std::io::Stdout`] for communicating with Fleetspeak.
///
/// Writing to this communication channel is not synchronized nor buffered.
pub struct CommsOutRaw {
    /// File handle of the output channel passed by the Fleetspeak process.
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl CommsInRaw {

    /// Returns a [`CommsIn`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsInRaw, CommsEnvError> {
        Ok(CommsInRaw {
            handle: env_var_handle("FLEETSPEAK_COMMS_CHANNEL_INFD")?,
        })
    }
}

impl CommsOutRaw {

    /// Returns a [`CommsOut`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsOutRaw, CommsEnvError> {
        Ok(CommsOutRaw {
            handle: env_var_handle("FLEETSPEAK_COMMS_CHANNEL_OUTFD")?,
        })
    }
}

impl std::io::Read for CommsInRaw {

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buf_len = u32::try_from(buf.len())
            .map_err(|_| std::io::ErrorKind::InvalidInput)?;

        let mut count = std::mem::MaybeUninit::uninit();

        // SAFETY: We do not have any assumptons on `self.handle`. We usually
        // want it to be a valid file handle but since it is passed to us from
        // the parent process, we cannot guarantee that it actually is.
        //
        // And this is why things are a bit fuzzy when it comes to safety: MSDN
        // documentation for this function [1] does not explicitly mention what
        // happens if we pass it an invalid handle. However, we know that there
        // exists the `ERROR_INVALID_HANDLE` [2] error code and other functions
        // are explicitly documented (e.g. `FlushFileBuffers` [3]) to return it
        // in case the handle is invalid. Moreover, from empirical study we know
        // that it is the case for `ReadFile` as well.
        //
        // The rest is just a function call as described in the docs: we pass a
        // valid buffer and the number of bytes we want to read (which we first
        // verify to fit the `u32` type required by the API). After the call we
        // check whether it succeeded.
        //
        // [1]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile
        // [2]: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
        // [3]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers
        let status = unsafe {
            windows_sys::Win32::Storage::FileSystem::ReadFile(
                self.handle,
                // TODO(@panhania): Upgrade `windows-sys` crate and remove cast.
                buf.as_mut_ptr().cast::<std::ffi::c_void>(),
                buf_len,
                count.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };

        if status == windows_sys::Win32::Foundation::FALSE {
            return Err(std::io::Error::last_os_error());
        }

        // SAFETY: We verified that the call to `ReadFile` succeeded and thus
        // `count` is guaranteed to be initialized to the number of bytes that
        // were read.
        let count = unsafe { count.assume_init() };

        Ok(count as usize)
    }
}

impl std::io::Write for CommsOutRaw {

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf_len = u32::try_from(buf.len())
            .map_err(|_| std::io::ErrorKind::InvalidInput)?;

        let mut count = std::mem::MaybeUninit::uninit();

        // SAFETY: We do not have any assumptons on `self.handle`. We usually
        // want it to be a valid file handle but since it is passed to us from
        // the parent process, we cannot guarantee that it actually is.
        //
        // And this is why things are a bit fuzzy when it comes to safety: MSDN
        // documentation for this function [1] does not explicitly mention what
        // happens if we pass it an invalid handle. However, we know that there
        // exists the `ERROR_INVALID_HANDLE` [2] error code and other functions
        // are explicitly documented (e.g. `FlushFileBuffers` [3]) to return it
        // in case the handle is invalid. Moreover, from empirical study we know
        // that it is the case for `WriteFile` as well.
        //
        // The rest is just a function call as described in the docs: we pass a
        // valid buffer and the number of bytes we want to write (which we first
        // verify to fit the `u32` type required by the API). After the call we
        // check whether it succeeded.
        //
        // [1]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile
        // [2]: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
        // [3]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers
        let status = unsafe {
            windows_sys::Win32::Storage::FileSystem::WriteFile(
                self.handle,
                buf.as_ptr(),
                buf_len,
                count.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };

        if status == windows_sys::Win32::Foundation::FALSE {
            return Err(std::io::Error::last_os_error());
        }

        // SAFETY: We verified that the call to `WriteFile` succeeded and thus
        // `count` is guaranteed to be initialized to the number of bytes that
        // were written.
        let count = unsafe { count.assume_init() };

        Ok(count as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // SAFETY: We do not have any assumptons on `self.handle`. We usually
        // want it to be a valid file handle but since it is passed to use from
        // the parent process, we cannot guarantee that it actually is.
        //
        // However, there is no unsafety here: in case the handle is not valid,
        // this function will cause `ERROR_INVALID_HANDLE` to be raised [1]. We
        // verify the status after the call.
        //
        // [1]: https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers
        let status = unsafe {
            windows_sys::Win32::Storage::FileSystem::FlushFileBuffers(
                self.handle,
            )
        };

        if status == windows_sys::Win32::Foundation::FALSE {
            return Err(std::io::Error::last_os_error());
        };

        Ok(())
    }
}

/// Retrieves a file handle specified in the given environment variable.
fn env_var_handle<K>(key: K) -> Result<windows_sys::Win32::Foundation::HANDLE, CommsEnvError>
where
    K: AsRef<std::ffi::OsStr>,
{
    match std::env::var(key) {
        Ok(string) => match string.parse() {
            Ok(handle) => Ok(handle),
            Err(_) => Err(CommsEnvError {
                repr: CommsEnvErrorRepr::NotParsable(string.into()),
            }),
        }
        Err(std::env::VarError::NotPresent) => Err(CommsEnvError {
            repr: CommsEnvErrorRepr::NotSpecified,
        }),
        Err(std::env::VarError::NotUnicode(string)) => Err(CommsEnvError {
            repr: CommsEnvErrorRepr::NotParsable(string),
        }),
    }
}
