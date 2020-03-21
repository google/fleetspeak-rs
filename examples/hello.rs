// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use fleetspeak::client::Packet;

fn main() -> std::io::Result<()> {
    fleetspeak::client::startup("0.0.1")?;

    loop {
        let request = fleetspeak::client::receive::<String>()?.data;
        let response = format!("Hello {}!", request);
        fleetspeak::client::send(Packet {
            service: String::from("greeter"),
            kind: None,
            data: response,
        })?;
    }
}
