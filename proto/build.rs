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

    protobuf_codegen_pure::run(protobuf_codegen_pure::Args {
        out_dir: &proto_out_dir.to_str().unwrap(),
        includes: &[
            "vendor/fleetspeak/fleetspeak/src",
            "vendor/protobuf/src",
        ],
        input: &[
            "vendor/fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto",
            "vendor/fleetspeak/fleetspeak/src/client/channel/proto/fleetspeak_channel/channel.proto",
        ],
        customize: protobuf_codegen_pure::Customize {
            // gen_mod_rs: Some(true),
            inside_protobuf: Some(true),
            ..Default::default()
        },
        ..Default::default()
    })?;

    // TODO: In newer versions of the `protobuf-codegen` there is a `gen_mod_rs`
    // setting that can be used to generate this file. Once we can migrate to
    // these newer versions, this inelegant hack should be removed.
    std::fs::write(proto_out_dir.join("mod.rs"), b"
        pub mod common;
        pub mod channel;
    ")?;

    Ok(())
}
