/*******************************************************************************
*   (c) 2021 Zondax GmbH
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
#![allow(dead_code)]

#[macro_use]
extern crate cfg_if;

/// cbindgen:ignore
pub(self) mod bindings {
    extern "C" {
        cfg_if! {
            if #[cfg(zemu_sdk)] {
                pub fn zemu_log(buffer: *const u8);
                pub fn zemu_log_stack(ctx: *const u8);
                pub fn check_canary();
            }
        }
    }
}

#[inline(always)]
pub fn zemu_log(_s: &str) {
    cfg_if! {
        if #[cfg(all(zemu_sdk, zemu_logging))] {
                unsafe {
                    let s = bolos_sys::pic::PIC::new(_s).into_inner();
                    let p = s.as_bytes().as_ptr();
                    bindings::zemu_log(p)
                }
        } else if #[cfg(zemu_sdk)] {
            //do nothing, logging not enabled
        } else {
            //polyfill for testing
            extern crate std;
            let s = _s.split_at(_s.len() - 1).0; // remove null termination
            std::print!("{}", s)
        }
    }
}

#[inline(always)]
pub fn zemu_log_stack(_s: &str) {
    #[cfg(all(zemu_sdk, zemu_logging))]
    unsafe {
        let _s = bolos_sys::pic::PIC::new(_s).into_inner();
        let p = _s.as_bytes().as_ptr();
        bindings::zemu_log_stack(p)
    }
}

pub fn check_canary() {
    #[cfg(zemu_sdk)]
    unsafe {
        bindings::check_canary();
    }
}

#[cfg_attr(not(zemu_sdk), path = "ui_mock.rs")]
mod ui;
pub use ui::*;

mod ui_toolkit;
