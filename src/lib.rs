//! Myros (**My** **R**ust **O**perating **S**ystem) is a hobby operating system being written
//! in Rust.
//!
//! Copyright 2019 Mike Leany
//!
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//!
//!>    <http://www.apache.org/licenses/LICENSE-2.0>
//!
//! Unless required by applicable law or agreed to in writing, software
//! distributed under the License is distributed on an "AS IS" BASIS,
//! WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//! See the License for the specific language governing permissions and
//! limitations under the License.

#![no_std]
#![feature(panic_info_message)]
use core::panic::PanicInfo;

/// Replaces the panic handler from the standard library which is not available
/// when using `#![no_std]` in a binary.
///
/// Does not return.
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> ! {
    halt();
}

/// Halt execution. If halting due to a failure, use `panic` instead.
///
/// Does not return.
pub fn halt() -> ! {
    loop {}
}
