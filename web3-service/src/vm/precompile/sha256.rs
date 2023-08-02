// SPDX-License-Identifier: Apache-2.0
// This file is part of Frontier.
//
// Copyright (c) 2020 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use {
    super::{ensure_linear_cost, PrecompileResult},
    evm::executor::stack::PrecompileOutput,
    evm_runtime::{Context, ExitSucceed},
    sha2::Digest,
};
pub struct Sha256;

impl Sha256 {
    const BASE: u64 = 60;
    const WORD: u64 = 12;

    pub fn execute(
        input: &[u8],
        gas_limit: Option<u64>,
        _context: &Context,
        _is_static: bool,
    ) -> PrecompileResult {
        let ret = sha2_256(input);
        let cost = ensure_linear_cost(gas_limit, input.len() as u64, Self::BASE, Self::WORD)?;

        Ok(PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost,
            output: ret.to_vec(),
            logs: vec![],
        })
    }
}
/// Do a sha2 256-bit hash and return result.
pub fn sha2_256(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let mut output = [0u8; 32];
    output.copy_from_slice(&hasher.finalize());
    output
}
