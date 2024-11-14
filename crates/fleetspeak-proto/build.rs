// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::path::PathBuf;

const PROTOS: &'static [&'static str] = &[
    "vendor/fleetspeak/fleetspeak/src/common/proto/fleetspeak/common.proto",
    "vendor/fleetspeak/fleetspeak/src/client/channel/proto/fleetspeak_channel/channel.proto",
];

fn main() {
    let outdir: PathBuf = std::env::var("OUT_DIR")
        .expect("no output directory")
        .into();

    for proto in PROTOS {
        println!("cargo:rerun-if-changed={}", proto);
    }

    let proto_out_dir = outdir.join("proto");
    std::fs::create_dir_all(&proto_out_dir).unwrap();

    let customize = protobuf_codegen::Customize::default()
        .gen_mod_rs(true)
        .generate_accessors(true);

    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir(&proto_out_dir)
        .include("vendor/fleetspeak/fleetspeak/src")
        .inputs(PROTOS)
        .customize(customize)
        .run().unwrap();
}
