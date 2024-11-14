// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt as _, WriteBytesExt as _};

use crate::Message;

#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "windows")]
mod windows;

mod sys {
    #[cfg(target_family = "unix")]
    pub use crate::io::unix::*;

    #[cfg(target_family = "windows")]
    pub use crate::io::windows::*;
}

pub use self::sys::{
    CommsInRaw,
    CommsOutRaw,
};

/// An error returned in case instantiating communicaton channels fails.
#[derive(Clone, Debug)]
pub struct CommsEnvError {
    repr: CommsEnvErrorRepr,
}

#[derive(Clone, Debug)]
enum CommsEnvErrorRepr {
    /// Communication channel is not specified in the environment.
    NotSpecified,
    /// Communication channel specified in the environment is not valid.
    NotParsable(std::ffi::OsString),
}

impl std::fmt::Display for CommsEnvError {

    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            CommsEnvErrorRepr::NotSpecified => {
                write!(fmt, "communication channel not specified")
            }
            CommsEnvErrorRepr::NotParsable(value) => {
                write!(fmt, "invalid communication channel value: {value:?}")
            }
        }
    }
}

impl std::error::Error for CommsEnvError {
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

/// Writes a Fleetspeak heartbeat record to the output buffer.
///
/// All client services should heartbeat from time to time. Otherwise, from
/// the Fleetspeak perspective, the service is unresponsive and should be
/// restarted.
///
/// The exact frequency of the required heartbeat is defined in the service
/// configuration file.
pub fn write_heartbeat<W>(output: &mut W) -> std::io::Result<()>
where
    W: Write,
{
    let mut proto = fleetspeak_proto::common::Message::new();
    proto.set_message_type(String::from("Heartbeat"));
    proto.mut_destination().set_service_name(String::from("system"));

    write_proto(output, proto)
}

/// Writes a Fleetspeak startup record to the output buffer.
///
/// All clients are required to send this information on startup. If the
/// client does not receive this information quickly enough, the service
/// will be killed.
///
/// The `version` string should contain a self-reported version of the
/// service. This data is used primarily for statistics.
pub fn write_startup<W>(output: &mut W, version: &str) -> std::io::Result<()>
where
    W: Write,
{
    let mut data = fleetspeak_proto::channel::StartupData::new();
    data.set_pid(i64::from(std::process::id()));
    data.set_version(String::from(version));

    let mut proto = fleetspeak_proto::common::Message::new();
    proto.set_message_type(String::from("StartupData"));
    proto.mut_destination().set_service_name(String::from("system"));
    *proto.mut_data() = protobuf::well_known_types::any::Any::pack(&data)?;

    write_proto(output, proto)
}

/// Writes a Fleetspeak message to the output buffer.
///
/// The message is sent to the server-side `service` and tagged with the
/// `kind` type. Note that this message type is rather irrelevant for
/// Fleetspeak and it is up to the service what to do with this information.
pub fn write_message<W>(output: &mut W, message: Message) -> std::io::Result<()>
where
    W: Write,
{
    let mut proto = fleetspeak_proto::common::Message::new();
    proto.set_message_type(message.kind.unwrap_or_else(String::new));
    proto.mut_destination().set_service_name(message.service);
    // TODO: Consider a way of providing the type URL of the data being sent.
    proto.mut_data().value = message.data;

    write_proto(output, proto)
}

/// Reads a Fleetspeak message from the input buffer.
///
/// This function will block until there is a message to be read in the
/// input. Errors are reported in case of any I/O failure or if the read
/// message was malformed (e.g. it cannot be parsed to the expected type).
pub fn read_message<R>(input: &mut R) -> std::io::Result<Message>
where
    R: Read,
{
    let mut proto = read_proto(input)?;

    // While missing source address might not be considered a critical error
    // in most cases, for our own sanity we fail for such messages as well.
    // Allowing such behaviour might indicate a more severe problem with
    // Fleetspeak and ignoring it simply masks the issue. This might be
    // reconsidered in the future.
    //
    // We could also return a "catchable" error and only drop the message rather
    // than failing hard but not to introduce awkward error hierarchy and adding
    // a lot of complexity to the code without much benefit.
    let service = if proto.has_source() {
        proto.take_source().take_service_name()
    } else {
        use std::io::ErrorKind::InvalidData;
        return Err(std::io::Error::new(InvalidData, "missing source address"));
    };

    // It is not clear what is the best approach here. If there is no data,
    // should we error-out or return a default value? For the time being we
    // stick to the default approach, but if this proves to be not working
    // well in practice, it might be reconsidered.
    let data = if proto.has_data() {
        proto.take_data()
    } else {
        log::warn!("empty message from '{}'", service);
        Default::default()
    };

    Ok(Message {
        service: service,
        kind: Some(proto.message_type),
        data: data.value,
    })
}

/// Writes a raw Fleetspeak Protocol Buffers message to the output buffer.
///
/// This method does not perform any validation of the message being emitted
/// and assumes that all the required fields are present.
///
/// Note that this call will fail only if the message cannot be written to
/// the output or cannot be properly encoded but will succeed even if the
/// message is not what the server expects.
fn write_proto<W>(output: &mut W, proto: fleetspeak_proto::common::Message) -> std::io::Result<()>
where
    W: Write,
{
    use protobuf::Message as _;

    // Fleetspeak is not able to send messages bigger than 2 MiB anyway, so we
    // generally do not expect overflows here.
    let size = u32::try_from(proto.compute_size())
        .map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
        })?;

    output.write_u32::<LittleEndian>(size)?;
    proto.write_to_writer(output)?;
    write_magic(output)?;
    output.flush()?;

    Ok(())
}

/// Reads a raw Fleetspeeak Protocol Buffers message from the input buffer.
///
/// This function will block until there is a message to be read from the
/// input. It will fail in case of any I/O error or if the message cannot
/// be parsed as a Fleetspeak message.
fn read_proto<R>(input: &mut R) -> std::io::Result<fleetspeak_proto::common::Message>
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
fn write_magic<W>(output: &mut W) -> std::io::Result<()>
where
    W: Write,
{
    output.write_u32::<LittleEndian>(MAGIC)?;

    Ok(())
}

/// Reads the Fleetspeak magic from the input buffer.
fn read_magic<R>(input: &mut R) -> std::io::Result<()>
where
    R: Read,
{
    let magic = input.read_u32::<LittleEndian>()?;
    if magic != MAGIC {
        return Err(InvalidMagicError { magic }.into());
    }

    Ok(())
}

/// Invalid magic number was read from the input stream.
#[derive(Debug)]
struct InvalidMagicError {
    /// Invalid magic that was read from the input stream.
    magic: u32,
}

impl std::fmt::Display for InvalidMagicError {

    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
}
