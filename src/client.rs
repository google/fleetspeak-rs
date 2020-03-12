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
use std::sync::Mutex;

pub struct Connection<R, W> {
    pub input: R,
    pub output: W,
}

impl<R: Read, W: Write> Connection<R, W> {

    pub fn new(input: R, output: W) -> Result<Self> {
        let mut conn = Connection {
            input: input,
            output: output,
        };
        conn.handshake()?;

        Ok(conn)
    }

    pub fn heartbeat(&mut self) -> Result<()> {
        let msg = Message {
            message_type: "Heartbeat".to_string(),
            destination: Some(Address {
                service_name: "system".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    pub fn startup(&mut self, version: &str) -> Result<()> {
        let data = StartupData {
            pid: std::process::id() as i64,
            version: version.to_string(),
        };

        let mut buf = Vec::new();
        prost::Message::encode(&data, &mut buf).map_err(invalid_data_error)?;

        let msg = Message {
            message_type: "StartupData".to_string(),
            destination: Some(Address {
                service_name: "system".to_string(),
                ..Default::default()
            }),
            data: Some(prost_types::Any {
                value: buf,
                type_url: "type.googleapis.com/fleetspeak.channel.StartupData".to_string(),
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    pub fn send<M>(&mut self, service: &str, kind: &str, data: M) -> Result<()>
    where
        M: prost::Message,
    {
        let mut buf = Vec::new();
        prost::Message::encode(&data, &mut buf).map_err(invalid_data_error)?;

        let msg = Message {
            message_type: kind.to_string(),
            destination: Some(Address {
                service_name: service.to_string(),
                ..Default::default()
            }),
            data: Some(prost_types::Any {
                value: buf.to_vec(),
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
        let mut buf = Vec::new();
        prost::Message::encode(&msg, &mut buf).map_err(invalid_data_error)?;

        self.output.write_u32::<LittleEndian>(buf.len() as u32)?;
        self.output.write(&buf)?;
        self.output.write_u32::<LittleEndian>(MAGIC)?;
        self.output.flush()?;

        Ok(())
    }

    fn collect(&mut self) -> Result<Message> {
        let len = self.input.read_u32::<LittleEndian>()? as usize;
        let mut buf = vec!(0; len);
        self.input.read_exact(&mut buf[..])?;

        let magic = self.input.read_u32::<LittleEndian>()?;
        if magic != MAGIC {
            let err = invalid_data_error(format!("invalid magic: `{}`", magic));
            return Err(err);
        }

        prost::Message::decode(&buf[..]).map_err(invalid_data_error)
    }

    fn handshake(&mut self) -> Result<()> {
        self.output.write_u32::<LittleEndian>(MAGIC)?;
        self.output.flush()?;

        let magic = self.input.read_u32::<LittleEndian>()?;
        if magic != MAGIC {
            let err = invalid_data_error(format!("invalid magic `{}`", magic));
            return Err(err);
        }

        Ok(())
    }

}

fn invalid_data_error<E>(err: E) -> std::io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    return std::io::Error::new(std::io::ErrorKind::InvalidData, err);
}

const MAGIC: u32 = 0xf1ee1001;

fn open(var: &str) -> std::fs::File {
    let fd = match std::env::var(var) {
        Ok(fd) => fd,
        Err(err) => panic!("invalid variable `{}`: {}", var, err),
    };

    let fd = match fd.parse() {
        Ok(fd) => fd,
        Err(err) => panic!("failed to parse a file descriptor: {}", err),
    };

    // TODO: Add support for Windows.
    unsafe {
        std::os::unix::io::FromRawFd::from_raw_fd(fd)
    }
}

lazy_static! {
    static ref CONNECTION: Mutex<Connection<std::fs::File, std::fs::File>> = {
        let input = open("FLEETSPEAK_COMMS_CHANNEL_INFD");
        let output = open("FLEETSPEAK_COMMS_CHANNEL_OUTFD");

        let conn = Connection::new(input, output).expect("handshake failure");
        Mutex::new(conn)
    };
}

pub fn heartbeat() -> Result<()> {
    connected(|conn| conn.heartbeat())
}

pub fn startup(version: &str) -> Result<()> {
    connected(|conn| conn.startup(version))
}

pub fn send<M>(service: &str, kind: &str, data: M) -> Result<()>
where
    M: prost::Message,
{
    connected(|conn| conn.send(service, kind, data))
}

pub fn receive<M>() -> Result<M>
where
    M: prost::Message + Default,
{
    connected(|conn| conn.receive())
}

fn connected<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut Connection<std::fs::File, std::fs::File>) -> Result<T>
{
    let mut conn = CONNECTION.lock().expect("poisoned connection mutex");
    f(&mut conn)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn handshake_good_magic() {
        let mut buf_in = [0; 1024];
        let mut buf_out = [0; 1024];

        let mut cur = Cursor::new(&mut buf_in[..]);
        assert!(cur.write_u32::<LittleEndian>(MAGIC).is_ok());

        let cur_in = Cursor::new(&mut buf_in[..]);
        let cur_out = Cursor::new(&mut buf_out[..]);
        assert!(Connection::new(cur_in, cur_out).is_ok());

        let mut cur = Cursor::new(&mut buf_out[..]);
        assert_eq!(cur.read_u32::<LittleEndian>().unwrap(), MAGIC);
    }

    #[test]
    fn handshake_bad_magic() {
        let mut buf_in = [0; 1024];
        let mut buf_out = [0; 1024];

        let mut cur = Cursor::new(&mut buf_in[..]);
        assert!(cur.write_u32::<LittleEndian>(0xf1ee1337).is_ok());

        let cur_in = Cursor::new(&mut buf_in[..]);
        let cur_out = Cursor::new(&mut buf_out[..]);
        assert!(Connection::new(cur_in, cur_out).is_err());
    }
}
