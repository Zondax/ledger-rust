/*******************************************************************************
*   (c) 2022 Zondax AG
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
#![no_std]
#![no_builtins]

extern crate no_std_compat as std;
extern crate self as bolos;

#[macro_use]
extern crate cfg_if;

pub use bolos_derive::*;

#[macro_use]
pub mod flash_slot;
pub use flash_slot::Wear;

#[macro_use]
pub mod swapping_buffer;
pub use swapping_buffer::SwappingBuffer;

pub mod lock;
pub use lock::Lock;

pub mod uploader;
pub use uploader::Uploader;

pub mod wrapper;
pub use wrapper::ApduBufferRead;

mod panic_traits;
pub use panic_traits::LedgerUnwrap;

mod apdu_errors;
pub use apdu_errors::ApduError;

mod utils;
pub use utils::*;

/// Set of utilities for UI
pub mod ui;

/// Contains the necessary mechanisms to
/// register the app's handlers to be used.
/// The registration requires 2 items,
/// the instruction code used to determine if the handler should be called
/// and the handler function itself.
///
/// # How to use
/// To register the app handler you should use the [`handlers::register`] macro:
/*
```rust
# use bolos::handlers::*;
# use bolos::{ApduBufferRead, ApduError};

struct Version;
impl ApduHandler for Version {
    fn handle(_: &mut u32, _: ApduBufferRead) -> Result<u32, ApduError> {
        Ok(0)
    }
}

#[register(HANDLERS)]
static APP_VERSION: (u8, HandlerFn) = (0, Version::handle);

# assert_eq!(unsafe{ HANDLERS.len() }, 1);
```
*/
pub mod handlers;
pub use handlers::{apdu_dispatch, ApduHandler};

cfg_if! {
    if #[cfg(all(__impl, __mock))] {
        compiler_error!("Can't have both `__impl` and `__mock` enabled");
    } else if #[cfg(__impl)] {
        pub use bolos_impl::*;
    } else if #[cfg(__mock)] {
        pub use bolos_mock::*;
    } else {
        compile_error!("Need either `__impl` or `__mock` feature enabled");
    }
}
