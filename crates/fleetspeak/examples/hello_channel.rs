// Copyright 2026 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

// TODO(@panhania): Make this the default `hello` example.

use std::time::Duration;

use fleetspeak::Message;

fn main() {
    let fleetspeak = fleetspeak::Comms::from_env()
        .expect("failed to handshake Fleetspeak connection");

    fleetspeak.startup("0.0.1")
        .expect("failed to send Fleetspeak startup message");

    while let Some(packet) = fleetspeak.try_receive_with_heartbeat(Duration::from_secs(1))
        .expect("failed to receive Fleetspeak message")
    {
        let request = std::str::from_utf8(&packet.data).unwrap();
        let response = format!("Hello, {}!", request);

        fleetspeak.send(Message {
            service: String::from("greeter"),
            kind: None,
            data: response.into_bytes(),
        }).expect("failed to send Fleetspeak message");
    }
}
