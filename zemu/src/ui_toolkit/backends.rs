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
use core::ops::DerefMut;

use crate::ui::{manual_vtable::RefMutDynViewable, Viewable};

use super::Zui;

pub trait UIBackend<const KEY_SIZE: usize>: Sized {
    type MessageBuf: DerefMut<Target = str>;

    //How many "action" items are we in charge of displaying also
    const INCLUDE_ACTIONS_COUNT: usize;

    fn static_mut() -> &'static mut Self;

    /// Retrieve the buffer where to write the title
    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE];

    /// Retrieve the buffer where to write the message
    fn message_buf(&mut self) -> Self::MessageBuf;

    /// Split the message into multiple lines if necessary
    fn split_value_field(&mut self, message_buf: Self::MessageBuf);

    /// Paint the main (idle) menu with the given page and status
    fn show_idle(&mut self, item_idx: usize, status: Option<&[u8]>);

    /// Paint the error screen
    fn show_error(&mut self);

    /// Paint the message screen
    fn show_message(&mut self, title: &str, message: &str);

    /// Paint the review screen (main entrypoint)
    fn show_review(ui: &mut Zui<Self, KEY_SIZE>);

    //h_review_update
    fn update_review(ui: &mut Zui<Self, KEY_SIZE>);

    /// UX_WAIT macro equivalent
    fn wait_ui(&mut self);

    /// Retrieve current expert status
    fn expert(&self) -> bool;

    /// Toggle the expert status and trigger ui refresh
    fn toggle_expert(&mut self);

    /// Update the display to show the changed expert status
    fn update_expert(&mut self);

    /// Retrieve the buffer to use as output for UI's accept or reject
    fn accept_reject_out(&mut self) -> &mut [u8];

    /// Signal the UI that the result has been written, with `len` bytes used
    fn accept_reject_end(&mut self, len: usize);

    /// Store the given `Viewable` and return the new handle to interact with it
    fn store_viewable<V: Viewable + Sized + 'static>(
        &mut self,
        viewable: V,
    ) -> Option<RefMutDynViewable>;
}

cfg_if::cfg_if! {
    if #[cfg(any(nanos, feature = "cbindgen_s"))] {
        mod nanos;
        pub use nanos::{NanoSBackend, RUST_ZUI};
    } else if #[cfg(any(nanox, feature = "cbindgen_x"))] {
        mod nanox;
        pub use nanox::{NanoXBackend, RUST_ZUI};
    } else if #[cfg(any(nanosplus, feature = "cbindgen_sp"))] {
        mod nanosplus;
        pub use nanosplus::{NanoSPBackend, RUST_ZUI};
    } else if #[cfg(any(stax, feature = "cbindgen_fs"))] {
        mod stax;
        pub use stax::{StaxBackend, RUST_ZUI};
    } else {
        mod console;
        pub use console::{ConsoleBackend, RUST_ZUI};
    }
}
