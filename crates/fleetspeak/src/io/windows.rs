// Copyright 2024 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use super::CommsEnvError;

/// Alternative for [`std::io::Stdin`] for communicating with Fleetspeak.
pub struct CommsIn {
    // TODO(@panhania): Use raw pipes.
}

/// Alternative for [`std::io::Stdout`] for communicating with Fleetspeak.
pub struct CommsOut {
    // TODO(@panhania): Use raw pipes.
}

impl CommsIn {

    /// Returns a [`CommsIn`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsIn, CommsEnvError> {
        todo!()
    }
}

impl CommsOut {

    /// Returns a [`CommsOut`] instance given by the parent Fleetspeak process.
    pub fn from_env() -> Result<CommsOut, CommsEnvError> {
        todo!()
    }
}

impl std::io::Read for CommsIn {

    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl std::io::Write for CommsOut {

    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        todo!()
    }
}
