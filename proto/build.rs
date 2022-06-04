// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

// extern crate prost_build;
use std::io::Result;
use std::path::PathBuf;

const PROTOS: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto",
    "fleetspeak/fleetspeak/src/client/channel/proto/fleetspeak_channel/channel.proto",
];

const INCLUDES: &'static [&'static str] = &[
    "fleetspeak/fleetspeak/src",
];

fn cargo_out_dir() -> PathBuf {
    let out_dir = std::env::var("OUT_DIR")
        .expect("output folder not specified");
    
    PathBuf::from(out_dir)
}

fn proto_out_dir() -> PathBuf {
    cargo_out_dir().join("proto")
}

fn main() -> Result<()> {
    let proto_out_dir = proto_out_dir();
    std::fs::create_dir_all(&proto_out_dir)?;

    protobuf_codegen_pure::run(protobuf_codegen_pure::Args {
        out_dir: &proto_out_dir.to_str().unwrap(),
        includes: INCLUDES,
        input: PROTOS,
        customize: protobuf_codegen_pure::Customize {
            gen_mod_rs: Some(true),
            ..Default::default()
        },
    })?;

    prost_build::compile_protos(PROTOS, INCLUDES)
}
