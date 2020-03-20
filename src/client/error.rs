// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

pub enum ReadError {
    Input(std::io::Error),
    Decode(prost::DecodeError),
    Magic(u32),
}

pub enum WriteError {
    Output(std::io::Error),
    Encode(prost::EncodeError),
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
