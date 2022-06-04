// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::time::Duration;

use protobuf::well_known_types::StringValue;
use fleetspeak::Message;

fn main() -> std::io::Result<()> {
    fleetspeak::startup("0.0.1")?;

    loop {
        let packet = fleetspeak::receive_with_heartbeat(Duration::from_secs(1))?;

        let request: StringValue = packet.data;

        let mut response = StringValue::new();
        response.set_value(format!("Hello {}!", request.get_value()));

        fleetspeak::send(Message {
            service: String::from("greeter"),
            kind: None,
            data: response,
        })?;
    }
}
