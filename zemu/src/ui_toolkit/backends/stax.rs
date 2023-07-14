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
use super::UIBackend;
use crate::{
    ui::{apdu_buffer_mut, manual_vtable::RefMutDynViewable, store_into, ViewError, Viewable},
    ui_toolkit::Zui,
};
use bolos_derive::pic_str;
use bolos_sys::pic::PIC;

pub const KEY_SIZE: usize = 63 + 1;
//with null terminator
pub const MESSAGE_SIZE: usize = 4095 + 1;

pub const MAX_ITEMS: usize = 4;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: Zui<StaxBackend, KEY_SIZE> = Zui::new();

#[bolos_derive::lazy_static(cbindgen)]
static mut BACKEND: StaxBackend = StaxBackend::default();

const DEFAULT_IDLE: &[u8] = b"DO NOT USE\x00";

#[bolos_derive::lazy_static]
pub static mut IDLE_MESSAGE: *const u8 = core::ptr::null();

#[repr(C)]
struct Item {
    title: [u8; KEY_SIZE],
    message: [u8; MESSAGE_SIZE],
}

impl Default for Item {
    fn default() -> Self {
        Self {
            title: [0; KEY_SIZE],
            message: [0; MESSAGE_SIZE],
        }
    }
}

impl Item {
    pub fn reset_contents(&mut self) {
        self.title[0] = 0;
        self.message[0] = 0;
    }
}

#[repr(C)]
pub struct StaxBackend {
    // internal items buffers
    items: [Item; MAX_ITEMS],
    // tracks the number of items currently written to
    items_len: usize,

    // how big the current UI object is
    viewable_size: usize,
    expert_mode: bool,
}

impl Default for StaxBackend {
    fn default() -> Self {
        Self {
            items: Default::default(),
            items_len: 0,
            viewable_size: 0,
            expert_mode: false,
        }
    }
}

impl StaxBackend {
    pub fn reset_ui(&mut self) {
        for i in self.items.iter_mut() {
            i.reset_contents()
        }
        self.items_len = 0;
    }

    fn set_item(&mut self, idx: usize) {
        self.items_len = idx;
        self.items[idx].reset_contents();
    }

    pub fn advance_item(&mut self) -> bool {
        self.items_len += 1;
        if self.items.get(self.items_len).is_some() {
            true
        } else {
            false
        }
    }

    fn next_item_mut(&mut self) -> Option<&mut Item> {
        if self.advance_item() {
            self.current_item_mut()
        } else {
            None
        }
    }

    fn current_item_mut(&mut self) -> Option<&mut Item> {
        self.items.get_mut(self.items_len)
    }

    fn current_item(&self) -> Option<&Item> {
        self.items.get(self.items_len)
    }

    pub fn can_fit_item(&self) -> bool {
        self.current_item().is_some()
    }

    /// Set the current item to write to and launch update_review
    fn update_static_review(ui: &mut Zui<Self, KEY_SIZE>, idx: usize) -> bool {
        ui.backend.set_item(idx);
        Self::update_review(ui);
        true
    }
}

impl UIBackend<KEY_SIZE> for StaxBackend {
    type MessageBuf = &'static mut str;

    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

    //leave empty, no-op
    fn update_expert(&mut self) {}

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        let item = self
            .current_item_mut()
            //this shouldn't happen as we shouldn't get to here
            // unless we have enough slots
            // TODO: wraparound instead?
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });

        &mut item.title
    }

    fn message_buf(&mut self) -> Self::MessageBuf {
        // core::mem::drop(self);

        let item = Self::static_mut()
            .current_item_mut()
            //this shouldn't happen as we shouldn't get to here
            // unless we have enough slots
            // TODO: wraparound instead?
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });

        core::str::from_utf8_mut(&mut item.message)
            //this should never happen as we always asciify
            .unwrap_or_else(|_| unsafe { core::hint::unreachable_unchecked() })
    }

    //leave emtpy, no-op
    fn split_value_field(&mut self, _: &'static mut str) {}

    fn show_idle(&mut self, _item_idx: usize, status: Option<&[u8]>) {
        let status = status
            .or_else(|| unsafe {
                PIC::new(*IDLE_MESSAGE).into_inner().as_ref().map(|status| {
                    let len = crate::ui_toolkit::c_strlen(status, KEY_SIZE).unwrap_or(KEY_SIZE);

                    core::slice::from_raw_parts(status, len)
                })
            })
            .unwrap_or_else(|| PIC::new(DEFAULT_IDLE).into_inner());

        let mut item = &mut self.items[0];

        //truncate status
        let len = core::cmp::min(item.title.len() - 1, status.len());
        item.title[..len].copy_from_slice(&status[..len]);
        item.title[len] = 0; //0 terminate

        unsafe {
            bindings::crapolines::crapoline_home();
        }
    }

    fn show_error(&mut self) {
        unsafe {
            bindings::crapolines::crapoline_error();
        }
    }

    // put both title and message on the same item.message
    // as we displa only 1 text w/ spinner
    fn show_message(&mut self, title: &str, message: &str) {
        let item = &mut self.items[0];

        let title_len = core::cmp::min(item.message.len() - 1, title.len());
        item.message[..title_len].copy_from_slice(title[..title_len].as_bytes());

        if !message.is_empty() {
            let message_len = core::cmp::min(item.message.len() - title_len - 1, message.len());
            item.message[..message_len].copy_from_slice(message[..message_len].as_bytes());
        }

        unsafe {
            bindings::crapolines::crapoline_message();
        }
    }

    fn show_review(ui: &mut Zui<Self, KEY_SIZE>) {
        // initialize ui struct
        ui.paging_init();

        ui.backend.reset_ui();

        let is_address = ui
            .current_viewable
            .as_ref()
            .map(|v| v.is_address())
            .unwrap_or_default();

        if is_address {
            bindings::use_case_review_start(
                pic_str!("Verify address"),
                None,
                continuations::review_address,
            );
        } else {
            bindings::use_case_review_start(
                pic_str!("Review transaction"),
                None,
                continuations::review_transaction_static,
            );
        }
    }

    fn update_review(ui: &mut Zui<Self, KEY_SIZE>) {
        match ui.review_update_data() {
            Ok(_) => {
                ui.paging_increase();
            }
            Err(ViewError::NoData) => {}
            Err(_) => {
                ui.show_error();
            }
        }
    }

    //leave empty, no-op
    fn wait_ui(&mut self) {}

    fn expert(&self) -> bool {
        self.expert_mode
    }

    fn toggle_expert(&mut self) {
        self.expert_mode = !self.expert_mode;
    }

    fn accept_reject_out(&mut self) -> &mut [u8] {
        let buf = apdu_buffer_mut();
        let buf_len = buf.len();

        &mut buf[..buf_len - self.viewable_size]
    }

    fn accept_reject_end(&mut self, len: usize) {
        use bolos_sys::raw::{io_exchange, CHANNEL_APDU, IO_RETURN_AFTER_TX};

        // Safety: simple C call
        unsafe {
            io_exchange((CHANNEL_APDU | IO_RETURN_AFTER_TX) as u8, len as u16);
        }
    }

    fn store_viewable<V: Viewable + Sized + 'static>(
        &mut self,
        viewable: V,
    ) -> Option<RefMutDynViewable> {
        let (size, new_loc) = store_into(viewable, apdu_buffer_mut())?;
        self.viewable_size = size;
        Some(new_loc.into())
    }
}

mod cabi {
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn view_init_impl(msg: *const u8) {
        //this exists to force Lazy initialization from the C side
        *IDLE_MESSAGE = msg;
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_toggle_expert() {
        RUST_ZUI.backend.toggle_expert();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_expert() -> bool {
        RUST_ZUI.backend.expert()
    }

    #[no_mangle]
    pub unsafe extern "C" fn view_idle_show_impl(item_idx: u8, status: *const i8) {
        let status = if status.is_null() {
            None
        } else {
            let len = crate::ui_toolkit::c_strlen(status as *const u8, MESSAGE_SIZE)
                .unwrap_or(MESSAGE_SIZE);

            Some(core::slice::from_raw_parts(status as *const u8, len))
        };

        RUST_ZUI.show_idle(item_idx as usize, status)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_approve(_: cty::c_uint) {
        RUST_ZUI.approve();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_reject(_: cty::c_uint) {
        RUST_ZUI.reject();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_error_accept(_: cty::c_uint) {
        RUST_ZUI.accept_error();
    }

    /// This needs to refresh the UI with the content for the given item/pair
    ///
    /// Will also set which item to write to for this call
    ///
    /// Will be called by `nbgl_useCaseStaticReview`
    #[no_mangle]
    pub unsafe extern "C" fn rs_update_static_item(
        page: cty::uint8_t,
        internal_idx: cty::uint8_t,
    ) -> bool {
        if let Err(_) = RUST_ZUI.skip_to_item(page as usize) {
            return false;
        }

        StaxBackend::update_static_review(&mut RUST_ZUI, internal_idx as usize)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_action_callback(confirm: bool) {
        super::bindings::use_case_status("", confirm);
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_confirm_address(confirm: bool) {
        let message = if confirm {
            pic_str!("ADDRESS\nVERIFIED")
        } else {
            pic_str!("Address verification\ncancelled")
        };

        super::bindings::use_case_status(message, confirm);
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_confirm_txn(confirm: bool) {
        let message = if confirm {
            pic_str!("TRANSACTION\nSIGNED")
        } else {
            pic_str!("Transaction rejected")
        };

        super::bindings::use_case_status(message, confirm);
    }
}

mod bindings;

mod continuations {
    use super::{bindings, StaxBackend, UIBackend, RUST_ZUI};

    /// Continuation callback to kickoff the static review flow
    pub unsafe extern "C" fn review_transaction_static() {
        let total_pages = unsafe { RUST_ZUI.n_items() };

        bindings::use_case_static_review(total_pages as u8);
    }

    /// Continuation callback to kickoff an address flow
    pub unsafe extern "C" fn review_address() {
        let ui = unsafe { &mut *RUST_ZUI };
        let total_pages = ui.n_items();

        for idx in 0..total_pages {
            //always in the item range
            let _ = ui.skip_to_item(idx);
            ui.backend.set_item(idx);
            StaxBackend::update_review(&mut RUST_ZUI);
        }

        bindings::use_case_address(total_pages as u8);
    }
}
