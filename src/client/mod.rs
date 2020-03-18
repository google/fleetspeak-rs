// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.
mod connection;

use self::connection::Connection;

use lazy_static::lazy_static;
use std::fs::File;
use std::io::Result;
use std::sync::Mutex;

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
    F: FnOnce(&mut Connection<File, File>) -> Result<T>
{
    let mut conn = CONNECTION.lock().expect("poisoned connection mutex");
    f(&mut conn)
}

lazy_static! {
    static ref CONNECTION: Mutex<Connection<File, File>> = {
        let input = open("FLEETSPEAK_COMMS_CHANNEL_INFD");
        let output = open("FLEETSPEAK_COMMS_CHANNEL_OUTFD");

        let conn = Connection::new(input, output).expect("handshake failure");
        Mutex::new(conn)
    };
}

fn open(var: &str) -> File {
    let fd = std::env::var(var)
        .expect(&format!("invalid variable `{}`", var))
        .parse()
        .expect(&format!("failed to parse file descriptor"));

    // TODO: Add support for Windows.
    unsafe {
        std::os::unix::io::FromRawFd::from_raw_fd(fd)
    }
}
