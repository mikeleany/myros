//! Build script for Myros.
//
//  Copyright 2020 Mike Leany
// 
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
// 
//      <http://www.apache.org/licenses/LICENSE-2.0>
// 
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
///////////////////////////////////////////////////////////////////////////////////////////////////
#![warn(missing_docs, missing_debug_implementations, unused_extern_crates)]
#![warn(clippy::unimplemented, clippy::todo, clippy::unwrap_used)]

use nasm_rs;

fn main() {
    let src = "src/arch_x86_64/boot.asm";
    println!("cargo:rerun-if-changed={}", src);
    nasm_rs::compile_library_args("libboot.a", &[src], &["-felf64"]);
}
