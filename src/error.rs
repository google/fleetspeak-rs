// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::fmt::Formatter;

/// Invalid magic number was read from the input stream.
#[derive(Debug)]
pub(crate) struct InvalidMagicError {
    /// Invalid magic that was read from the input stream.
    pub(crate) magic: u32,
}

impl std::fmt::Display for InvalidMagicError {

    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "invalid Fleetspeak magic: 0x{:08x}", self.magic)
    }
}

impl std::error::Error for InvalidMagicError {
}

impl From<InvalidMagicError> for std::io::Error {

    fn from(error: InvalidMagicError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}
