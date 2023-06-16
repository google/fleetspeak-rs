// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::time::Duration;

use fleetspeak::Message;

fn main() {
    fleetspeak::startup("0.0.1");

    loop {
        let packet = fleetspeak::receive_with_heartbeat(Duration::from_secs(1));

        let request = std::str::from_utf8(&packet.data).unwrap();
        let response = format!("Hello, {}!", request);

        fleetspeak::send(Message {
            service: String::from("greeter"),
            kind: None,
            data: response.into_bytes(),
        });
    }
}
