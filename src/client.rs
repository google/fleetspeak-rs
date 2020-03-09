// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use byteorder::{LittleEndian, WriteBytesExt};
use fleetspeak_proto::common::{Message, Address};
use prost;
use prost_types;
use std::io::{Read, Write, Result, Error, ErrorKind};

pub struct Connection<'r, 'w, R, W> {
    pub input: &'r mut R,
    pub output: &'w mut W,
    buf: Vec<u8>,
}

impl<'r, 'w, R: Read, W: Write> Connection<'r, 'w, R, W> {

    pub fn new(input: &'r mut R, output: &'w mut W) -> Self {
        Connection {
            input: input,
            output: output,
            buf: vec!(0; 2 * 1024 * 1024), // TODO: Magic.
        }
    }

    pub fn send<M>(&mut self, service: &str, kind: &str, data: M) -> Result<()>
    where
        M: prost::Message,
    {
        self.encode(data)?;

        let msg = Message {
            message_type: kind.to_string(),
            destination: Some(Address {
                service_name: service.to_string(),
                ..Default::default()
            }),
            data: Some(prost_types::Any {
                value: self.buf.to_vec(),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    fn emit(&mut self, msg: Message) -> Result<()> {
        let len = prost::Message::encoded_len(&msg);
        self.encode(msg)?;

        self.output.write_u32::<LittleEndian>(len as u32)?;
        self.output.write(&self.buf[..len])?;
        self.output.write_u32::<LittleEndian>(0xf1ee1001)?; // TODO: Magic.

        Ok(())
    }

    fn encode<M>(&mut self, msg: M) -> Result<()>
    where
        M: prost::Message,
    {
        if let Err(err) = msg.encode(&mut self.buf) {
            let err = Error::new(ErrorKind::InvalidData, Box::new(err));
            return Err(err);
        };

        Ok(())
    }
}
