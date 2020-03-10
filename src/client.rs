// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fleetspeak_proto::common::{Message, Address};
use fleetspeak_proto::channel::{StartupData};
use lazy_static::lazy_static;
use prost;
use prost_types;
use std::io::{Read, Write, Result};
use std::marker::{Send, Sync};

pub struct Connection<R, W> {
    pub input: R,
    pub output: W,
    buf: Vec<u8>,
}

impl<R: Read, W: Write> Connection<R, W> {

    pub fn new(input: R, output: W) -> Self {
        Connection {
            input: input,
            output: output,
            buf: vec!(0; MAX_BUF_SIZE),
        }
    }

    pub fn heartbeat(&mut self) -> Result<()> {
        let msg = Message {
            message_type: "Hearbeat".to_string(),
            destination: Some(Address {
                service_name: "system".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    pub fn handshake(&mut self, version: &str) -> Result<()> {
        let data = StartupData {
            pid: std::process::id() as i64,
            version: version.to_string(),
        };

        self.send("system", "StartupData", data)
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

    pub fn receive<M>(&mut self) -> Result<M>
    where
        M: prost::Message + Default,
    {
        let msg = self.collect()?;

        // It is not clear what is the best approach here. If there is no data,
        // should we error-out or return a default value? For the time being we
        // stick to the default approach, but if this proves to be not working
        // well in practice, it might be reconsidered.
        let data = match msg.data {
            Some(data) => data,
            None => return Ok(Default::default()),
        };

        prost::Message::decode(&data.value[..]).map_err(invalid_data_error)
    }

    fn emit(&mut self, msg: Message) -> Result<()> {
        let len = prost::Message::encoded_len(&msg);
        self.encode(msg)?;

        self.output.write_u32::<LittleEndian>(len as u32)?;
        self.output.write(&self.buf[..len])?;
        self.output.write_u32::<LittleEndian>(MAGIC)?;

        Ok(())
    }

    fn collect(&mut self) -> Result<Message> {
        let len = self.input.read_i32::<LittleEndian>()? as usize;
        self.input.read_exact(&mut self.buf[..len])?;

        prost::Message::decode(&self.buf[..len]).map_err(invalid_data_error)
    }

    fn encode<M>(&mut self, msg: M) -> Result<()>
    where
        M: prost::Message,
    {
        msg.encode(&mut self.buf).map_err(invalid_data_error)
    }
}

fn invalid_data_error<E>(err: E) -> std::io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    return std::io::Error::new(std::io::ErrorKind::InvalidData, err);
}

const MAGIC: u32 = 0xf1ee1001;
const MAX_BUF_SIZE: usize = 2 * 1024 * 1024;
