// Copyright 2026 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

// TODO(#4): Make this the default `hello` example once the deprecated functions
// are removed.

use std::time::Duration;

use fleetspeak::Message;

fn main() {
    // SAFETY: We call `from_env` at the beginning of `main` so nothing could
    // have tampered with it. We also never invoke it again anywhere else.
    let fleetspeak = unsafe {
        fleetspeak::Comms::from_env()
    }.expect("failed to handshake Fleetspeak connection");

    fleetspeak.startup("0.0.1")
        .expect("failed to send Fleetspeak startup message");

    for packet in fleetspeak.receiver()
        .with_heartbeat(Duration::from_secs(1))
    {
        let packet = packet
            .expect("failed to receive Fleetspeak message");

        let request = std::str::from_utf8(&packet.data).unwrap();
        let response = format!("Hello, {}!", request);

        fleetspeak.send(Message {
            service: String::from("greeter"),
            kind: None,
            data: response.into_bytes(),
        }).expect("failed to send Fleetspeak message");
    }
}
