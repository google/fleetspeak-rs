// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use prost;
use prost_types;

use fleetspeak_proto::common::{Message, Address};
use fleetspeak_proto::channel::{StartupData};

use super::{ReadError, WriteError};

/// A Fleetspeak client connection object.
///
/// This connection owns two buffers: one for input and one for output. Usually,
/// one does not want to instantiate connection object themselves. Since the
/// Fleetspeak client will spawn the service process and provide it with file
/// descriptors to talk to, for user convenience there is a standard global
/// Fleetspeak client connection object that uses these descriptors.
pub struct Connection<R, W> {
    input: R,
    output: W,
}

/// A Fleetspeak client communication packet.
///
/// This structure represents incoming or outgoing packet objects delivered by
/// Fleetspeak. This is a simplified version of the underlying Protocol Buffers
/// message that exposes too much irrelevant fields and makes the protocol easy
/// to misuse.
pub struct Packet<M> {
    /// A name of the server-side service that sent or should receive the data.
    pub service: String,
    /// An optional message type that can be used by the server-side service.
    pub kind: Option<String>,
    /// A message to sent to the specified service.
    pub data: M,
}

impl<R: Read, W: Write> Connection<R, W> {

    /// Creates a new Fleetspeak connection.
    ///
    /// This function will perform a handshake procedure in order to verify
    /// correctness of the input and the output buffers. If the handshake
    /// procedure fails, an error is reported.
    pub fn new(input: R, output: W) -> std::io::Result<Self> {
        let mut conn = Connection {
            input: input,
            output: output,
        };
        conn.handshake()?;

        Ok(conn)
    }

    /// Sends a heartbeat information through this connection.
    ///
    /// All client services should heartbeat from time to time. Otherwise, from
    /// the Fleetspeak perspective, the service is unresponsive and should be
    /// restarted.
    ///
    /// The exact frequency of the required heartbeat is defined in the service
    /// configuration file.
    pub fn heartbeat(&mut self) -> Result<(), WriteError> {
        let msg = Message {
            message_type: String::from("Heartbeat"),
            destination: Some(Address {
                service_name: String::from("system"),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    /// Sends the startup information through this connection.
    ///
    /// All clients are required to send this information on startup. If the
    /// client does not receive this information quickly enough, the service
    /// will be killed.
    ///
    /// The `version` string should contain a self-reported version of the
    /// service. This data is used primarily for statistics.
    pub fn startup(&mut self, version: &str) -> Result<(), WriteError> {
        let data = StartupData {
            pid: std::process::id() as i64,
            version: String::from(version),
        };

        let mut buf = Vec::new();
        prost::Message::encode(&data, &mut buf)?;

        let msg = Message {
            message_type: String::from("StartupData"),
            destination: Some(Address {
                service_name: String::from("system"),
                ..Default::default()
            }),
            data: Some(prost_types::Any {
                value: buf,
                type_url: String::from("type.googleapis.com/fleetspeak.channel.StartupData"),
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    /// Sends the message to the Fleetspeak server through this connection.
    ///
    /// The message is sent to the server-side `service` and tagged with the
    /// `kind` type. Note that this message type is rather irrelevant for
    /// Fleetspeak and it is up to the service what to do with this information.
    pub fn send<M>(&mut self, packet: Packet<M>) -> Result<(), WriteError>
    where
        M: prost::Message,
    {
        let mut buf = Vec::new();
        prost::Message::encode(&packet.data, &mut buf)?;

        let msg = Message {
            message_type: packet.kind.unwrap_or_else(String::new),
            destination: Some(Address {
                service_name: packet.service,
                ..Default::default()
            }),
            data: Some(prost_types::Any {
                value: buf,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.emit(msg)
    }

    /// Receives the message from the Fleetspeak server through this connection.
    ///
    /// This function will block until there is a message to be read in the
    /// input. Errors are reported in case of any I/O failure or if the read
    /// message was malformed (e.g. it cannot be parsed to the expected type).
    pub fn receive<M>(&mut self) -> Result<Packet<M>, ReadError>
    where
        M: prost::Message + Default,
    {
        let msg = self.accept()?;

        // While missing source address might not be consider a critical error
        // in most cases, for our own sanity we just disregard such messages.
        // Allowing such behaviour might indicate a more severe problem with
        // Fleetspeak and ignoring it simply masks the issue. This might be
        // reconsidered in the future.
        let service = match msg.source {
            Some(addr) => addr.service_name,
            None => return Err(ReadError::malformed("missing source address")),
        };

        // It is not clear what is the best approach here. If there is no data,
        // should we error-out or return a default value? For the time being we
        // stick to the default approach, but if this proves to be not working
        // well in practice, it might be reconsidered.
        let data = msg.data.unwrap_or_else(Default::default);

        Ok(Packet {
            service: service,
            kind: Some(msg.message_type),
            data: prost::Message::decode(&data.value[..])?
        })
    }

    /// Emits a raw Fleetspeak message to the server through this connection.
    ///
    /// This method does not perform any validation of the message being emitted
    /// and assumes that all the required fields are present.
    ///
    /// Note that this call will fail only if the message cannot be written to
    /// the output or cannot be properly encoded but will succeed even if the
    /// message is not what the server expects.
    fn emit(&mut self, msg: Message) -> Result<(), WriteError> {
        let mut buf = Vec::new();
        prost::Message::encode(&msg, &mut buf)?;

        self.output.write_u32::<LittleEndian>(buf.len() as u32)?;
        self.output.write(&buf)?;
        self.write_magic()?;
        self.output.flush()?;

        Ok(())
    }

    /// Accepts a raw Fleetspeeak message from this connection.
    ///
    /// This function will block until there is a message to be read from the
    /// input. It will fail in case of any I/O error or if the message cannot
    /// be parsed as a Fleetspeak message.
    fn accept(&mut self) -> Result<Message, ReadError> {
        let len = self.input.read_u32::<LittleEndian>()? as usize;
        let mut buf = vec!(0; len);
        self.input.read_exact(&mut buf[..])?;
        self.read_magic()?;

        Ok(prost::Message::decode(&buf[..])?)
    }

    /// Executes the handshake procedure.
    fn handshake(&mut self) -> std::io::Result<()> {
        self.write_magic()?;
        self.output.flush()?;
        self.read_magic()?;

        Ok(())
    }

    /// Writes the Fleetspeak magic to the output buffer.
    fn write_magic(&mut self) -> Result<(), WriteError> {
        self.output.write_u32::<LittleEndian>(MAGIC)?;

        Ok(())
    }

    /// Reads the Fleetspeak magic from the input buffer.
    fn read_magic(&mut self) -> Result<(), ReadError> {
        let magic = self.input.read_u32::<LittleEndian>()?;
        if magic != MAGIC {
            return Err(ReadError::Magic(magic));
        }

        Ok(())
    }
}

const MAGIC: u32 = 0xf1ee1001;

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
