// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::{ReadError, WriteError};

/// A Fleetspeak client communication message.
///
/// This structure represents incoming or outgoing message objects delivered by
/// Fleetspeak. This is a simplified version of the underlying Protocol Buffers
/// message that exposes too much irrelevant fields and makes the protocol easy
/// to misuse.
pub struct Message<M: protobuf::Message> {
    /// A name of the server-side service that sent or should receive the data.
    pub service: String,
    /// An optional message type that can be used by the server-side service.
    pub kind: Option<String>,
    /// A message to sent to the specified service.
    pub data: M,
}

/// Executes the handshake procedure.
///
/// The handshake procedure consists of writing and reading magic numbers from
/// the connection buffers. This validates that the communication between the
/// Fleetspeak client and the service daemon is working as expected.
///
/// All Fleetspeak connection buffers are required to perform the handshake
/// before they became usable for sending and receiving messages.
pub fn handshake<R, W>(input: &mut R, output: &mut W) -> std::io::Result<()>
where
    R: Read,
    W: Write,
{
    write_magic(output)?;
    output.flush()?;
    read_magic(input)?;

    Ok(())
}

/// Sends a heartbeat information through this connection.
///
/// All client services should heartbeat from time to time. Otherwise, from
/// the Fleetspeak perspective, the service is unresponsive and should be
/// restarted.
///
/// The exact frequency of the required heartbeat is defined in the service
/// configuration file.
pub fn heartbeat<W>(output: &mut W) -> Result<(), WriteError>
where
    W: Write,
{
    let mut msg = fleetspeak_proto::common::Message::new();
    msg.set_message_type(String::from("Heartbeat"));
    msg.mut_destination().set_service_name(String::from("system"));

    write_raw(output, msg)
}

/// Sends the startup information through this connection.
///
/// All clients are required to send this information on startup. If the
/// client does not receive this information quickly enough, the service
/// will be killed.
///
/// The `version` string should contain a self-reported version of the
/// service. This data is used primarily for statistics.
pub fn startup<W>(output: &mut W, version: &str) -> Result<(), WriteError>
where
    W: Write,
{
    use protobuf::Message as _;

    let mut data = fleetspeak_proto::channel::StartupData::new();
    data.set_pid(i64::from(std::process::id()));
    data.set_version(String::from(version));

    let mut msg = fleetspeak_proto::common::Message::new();
    msg.set_message_type(String::from("StartupData"));
    msg.mut_destination().set_service_name(String::from("system"));
    msg.mut_data().set_type_url(type_url(&data));
    msg.mut_data().set_value(data.write_to_bytes()?);

    write_raw(output, msg)
}

/// Sends the message to the Fleetspeak server through the output buffer.
///
/// The message is sent to the server-side `service` and tagged with the
/// `kind` type. Note that this message type is rather irrelevant for
/// Fleetspeak and it is up to the service what to do with this information.
pub fn send<W, M>(output: &mut W, message: Message<M>) -> Result<(), WriteError>
where
    W: Write,
    M: protobuf::Message,
{
    let mut proto = fleetspeak_proto::common::Message::new();
    proto.set_message_type(message.kind.unwrap_or_else(String::new));
    proto.mut_destination().set_service_name(message.service);
    proto.mut_data().set_type_url(type_url(&message.data));
    proto.mut_data().set_value(message.data.write_to_bytes()?);

    write_raw(output, proto)
}

/// Receives the message from the Fleetspeak server through the input buffer.
///
/// This function will block until there is a message to be read in the
/// input. Errors are reported in case of any I/O failure or if the read
/// message was malformed (e.g. it cannot be parsed to the expected type).
pub fn receive<R, M>(input: &mut R) -> Result<Message<M>, ReadError>
where
    R: Read,
    M: protobuf::Message,
{
    let mut msg = read_raw(input)?;

    // While missing source address might not be consider a critical error
    // in most cases, for our own sanity we just disregard such messages.
    // Allowing such behaviour might indicate a more severe problem with
    // Fleetspeak and ignoring it simply masks the issue. This might be
    // reconsidered in the future.
    let service = if msg.has_source() {
        msg.take_source().take_service_name()
    } else {
        return Err(ReadError::malformed("missing source address"))
    };

    // It is not clear what is the best approach here. If there is no data,
    // should we error-out or return a default value? For the time being we
    // stick to the default approach, but if this proves to be not working
    // well in practice, it might be reconsidered.
    let data = if msg.has_data() {
        msg.take_data()
    } else {
        log::warn!(target: "fleetspeak", "empty message from '{}'", service);
        Default::default()
    };

    Ok(Message {
        service: service,
        kind: Some(msg.message_type),
        data: protobuf::Message::parse_from_bytes(&data.value[..])?,
    })
}

/// Writes a raw Fleetspeak message to the output buffer.
///
/// This method does not perform any validation of the message being emitted
/// and assumes that all the required fields are present.
///
/// Note that this call will fail only if the message cannot be written to
/// the output or cannot be properly encoded but will succeed even if the
/// message is not what the server expects.
fn write_raw<W>(output: &mut W, msg: fleetspeak_proto::common::Message) -> Result<(), WriteError>
where
    W: Write,
{
    use protobuf::Message as _;

    output.write_u32::<LittleEndian>(msg.compute_size())?;
    msg.write_to_writer(output)?;
    write_magic(output)?;
    output.flush()?;

    Ok(())
}

/// Reads a raw Fleetspeeak message from the input buffer.
///
/// This function will block until there is a message to be read from the
/// input. It will fail in case of any I/O error or if the message cannot
/// be parsed as a Fleetspeak message.
fn read_raw<R>(input: &mut R) -> Result<fleetspeak_proto::common::Message, ReadError>
where
    R: Read,
{
    let len = input.read_u32::<LittleEndian>()? as usize;
    let mut buf = vec!(0; len);

    input.read_exact(&mut buf[..])?;
    read_magic(input)?;

    Ok(protobuf::Message::parse_from_bytes(&buf[..])?)
}

/// Writes the Fleetspeak magic to the output buffer.
fn write_magic<W>(output: &mut W) -> Result<(), WriteError>
where
    W: Write,
{
    output.write_u32::<LittleEndian>(MAGIC)?;

    Ok(())
}

/// Reads the Fleetspeak magic from the input buffer.
fn read_magic<R>(input: &mut R) -> Result<(), ReadError>
where
    R: Read,
{
    let magic = input.read_u32::<LittleEndian>()?;
    if magic != MAGIC {
        return Err(ReadError::Magic(magic));
    }

    Ok(())
}

const MAGIC: u32 = 0xf1ee1001;

/// Computes a type URL of the given Protocol Buffers message.
///
/// This function should probably be part of the `protobuf` package but for some
/// reason it is not and we have to implement it ourselves.
fn type_url<M: protobuf::Message>(message: &M) -> String {
    format!("{}/{}", TYPE_URL_PREFIX, message.descriptor().full_name())
}

const TYPE_URL_PREFIX: &'static str = "type.googleapis.com";

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

        let mut cur_in = Cursor::new(&mut buf_in[..]);
        let mut cur_out = Cursor::new(&mut buf_out[..]);
        assert!(handshake(&mut cur_in, &mut cur_out).is_ok());

        let mut cur = Cursor::new(&mut buf_out[..]);
        assert_eq!(cur.read_u32::<LittleEndian>().unwrap(), MAGIC);
    }

    #[test]
    fn handshake_bad_magic() {
        let mut buf_in = [0; 1024];
        let mut buf_out = [0; 1024];

        let mut cur = Cursor::new(&mut buf_in[..]);
        assert!(cur.write_u32::<LittleEndian>(0xf1ee1337).is_ok());

        let mut cur_in = Cursor::new(&mut buf_in[..]);
        let mut cur_out = Cursor::new(&mut buf_out[..]);
        assert!(handshake(&mut cur_in, &mut cur_out).is_err());
    }

    #[test]
    fn type_url_startup_data() {
        assert_eq! {
            type_url(&fleetspeak_proto::channel::StartupData::new()),
            "type.googleapis.com/fleetspeak.channel.StartupData"
        };
    }
}
