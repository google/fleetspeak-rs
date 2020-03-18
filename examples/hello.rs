// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

fn main() -> std::io::Result<()> {
    fleetspeak::client::startup("0.0.1")?;

    loop {
        let request = fleetspeak::client::receive::<String>()?;
        let response = format!("Hello {}!", request);
        fleetspeak::client::send("greeter", "greeting", response)?;
    }
}
