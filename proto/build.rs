extern crate prost_build;

use std::io::Result;

const PROTOS: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto",
];

const INCLUDES: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src",
];

fn main() -> Result<()> {
    prost_build::compile_protos(PROTOS, INCLUDES)
}
