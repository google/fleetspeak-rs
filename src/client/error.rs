// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::error::{Error};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ReadError {
    Input(std::io::Error),
    Decode(prost::DecodeError),
    Magic(u32),
}

#[derive(Debug)]
pub enum WriteError {
    Output(std::io::Error),
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
