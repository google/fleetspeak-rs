// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::error::{Error};
use std::fmt::{Display, Formatter};

/// An error type for failures that occurred when receiving a message.
#[derive(Debug)]
pub enum ReadError {
    /// An I/O error occurred when reading from the input stream.
    Input(std::io::Error),
    /// An error occurred when decoding bytes of the proto message.
    Decode(prost::DecodeError),
    /// An invalid magic number has been read from the input stream.
    Magic(u32),
}

/// An error type for failures that occured when sending a message.
#[derive(Debug)]
pub enum WriteError {
    /// An I/O error occurred when writing to the output stream.
    Output(std::io::Error),
    /// An error occurred when encoding the proto message to bytes.
    Encode(prost::EncodeError),
}

impl Display for ReadError {

    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        use ReadError::*;

        match *self {
            Input(ref err) => write!(fmt, "input error: {}", err),
            Decode(ref err) => write!(fmt, "proto decoding error: {}", err),
            Magic(magic) => write!(fmt, "invalid magic: {}", magic),
        }
    }
}

impl Display for WriteError {

    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        use WriteError::*;

        match *self {
            Output(ref err) => write!(fmt, "output error: {}", err),
            Encode(ref err) => write!(fmt, "proto encoding error: {}", err),
        }
    }
}

impl Error for ReadError {

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use ReadError::*;

        match *self {
            Input(ref err) => Some(err),
            Decode(ref err) => Some(err),
            Magic(_) => None,
        }
    }
}

impl Error for WriteError {

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use WriteError::*;

        match *self {
            Output(ref err) => Some(err),
            Encode(ref err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ReadError {

    fn from(err: std::io::Error) -> ReadError {
        ReadError::Input(err)
    }
}

impl From<prost::DecodeError> for ReadError {

    fn from(err: prost::DecodeError) -> ReadError {
        ReadError::Decode(err)
    }
}

impl From<std::io::Error> for WriteError {

    fn from(err: std::io::Error) -> WriteError {
        WriteError::Output(err)
    }
}

impl From<prost::EncodeError> for WriteError {

    fn from(err: prost::EncodeError) -> WriteError {
        WriteError::Encode(err)
    }
}

impl From<ReadError> for std::io::Error {

    fn from(err: ReadError) -> std::io::Error {
        use ReadError::*;

        match err {
            Input(err) => err,
            Decode(err) => err.into(),
            Magic(magic) => {
                let err = format!("invalid magic: {}", magic);
                std::io::Error::new(std::io::ErrorKind::InvalidData, err)
            },
        }
    }
}

impl From<WriteError> for std::io::Error {

    fn from(err: WriteError) -> std::io::Error {
        use WriteError::*;

        match err {
            Output(err) => err,
            Encode(err) => err.into(),
        }
    }
}
