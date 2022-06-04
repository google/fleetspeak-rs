// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::io::Result;
use std::path::PathBuf;

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

    protobuf_codegen_pure::Codegen::new()
        .out_dir(&proto_out_dir.to_str().unwrap())
        .include("fleetspeak/fleetspeak/src")
        .input("fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto")
        .input("fleetspeak/fleetspeak/src/client/channel/proto/fleetspeak_channel/channel.proto")
        .customize(protobuf_codegen_pure::Customize {
            gen_mod_rs: Some(true),
            ..Default::default()
        })
        .run()
}
