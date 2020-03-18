// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

extern crate prost_build;

use std::io::Result;

const PROTOS: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto",
    "fleetspeak/fleetspeak/src/client/channel/proto/fleetspeak_channel/channel.proto",
];

const INCLUDES: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src",
];

fn main() -> Result<()> {
    prost_build::compile_protos(PROTOS, INCLUDES)
}
