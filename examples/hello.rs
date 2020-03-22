// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::time::Duration;

use fleetspeak::Packet;

fn main() -> std::io::Result<()> {
    fleetspeak::startup("0.0.1")?;

    loop {
        let packet = fleetspeak::collect(Duration::from_secs(1))?;

        let request: String = packet.data;
        let response: String = format!("Hello {}!", request);

        fleetspeak::send(Packet {
            service: String::from("greeter"),
            kind: None,
            data: response,
        })?;
    }
}
