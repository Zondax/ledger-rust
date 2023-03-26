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
    ui::{apdu_buffer_mut, manual_vtable::RefMutDynViewable, ViewError, Viewable},
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

#[derive(Default)]
#[repr(C)]
struct Item {
    title: [u8; KEY_SIZE],
    message: [u8; MESSAGE_SIZE],
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

    // used to pass thru the current ui page content
    nbgl_page_content: *mut (),
}

impl Default for StaxBackend {
    fn default() -> Self {
        Self {
            items: Default::default(),
            items_len: 0,
            viewable_size: 0,
            nbgl_page_content: core::ptr::null_mut(),
        }
    }
}

impl Stax {
    pub fn reset_ui(&mut self) {
        self.items.reset();
        self.items_len = 0;
    }

    pub fn advance_item(&mut self) -> bool {
        self.items_len += 1;
        if self.items.get(self.items_len).is_some() {
            true
        } else {
            self.items_len -= 1;
            false
        }
    }

    pub fn next_item_mut(&mut self) -> Option<&mut Item> {
        self.advance_item()
            .then_some(|| ())
            .and_then(|_| self.current_item_mut())
    }

    pub fn current_item_mut(&mut self) -> Option<&mut Item> {
        self.items.get_mut(self.items_len)
    }
    pub fn current_item(&self) -> Option<&Item> {
        self.items.get(self.items_len)
    }

    pub fn can_fit_item(&self) -> bool {
        self.current_item().is_some()
    }
}

impl UIBackend<KEY_SIZE> for StaxBackend {
    type MessageBuf = &'static mut str;

    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

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

    fn message_buf(&mut self) -> &'static mut str {
        let item = self
            .current_item_mut()
            //this shouldn't happen as we shouldn't get to here
            // unless we have enough slots
            // TODO: wraparound instead?
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() });

        core::str::from_utf8_mut(&mut item.message)
            //this should never happen as we always asciify
            .unwrap_or_else(|| unsafe { core::hint::unreachable_unchecked() })
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

        let item = self.items[0];

        //truncate status
        let len = core::cmp::min(self.key.len() - 1, status.len());
        item.title[..len].copy_from_slice(status);
        item.title[len] = 0; //0 terminate

        unsafe {
            bindings::crapolines::crapoline_home();
        }
    }

    fn show_error(&mut self) {}

    fn show_message(&mut self, _title: &str, _message: &str) {}

    fn show_review(ui: &mut Zui<Self, KEY_SIZE>) {
        // initialize ui struct
        ui.paging_init();

        ui.backend.reset_ui();

        bindings::use_case_review_start(
            pic_str!("Review transaction"),
            None,
            continuations::review_transaction,
        );
    }

    fn update_review(ui: &mut Zui<Self, KEY_SIZE>) {
        let this = ui.backend;

        let mut n_items = 1;
        while this.can_fit_item() {
            this.advance_item();

            match ui.review_update_data() {
                Ok(_) => {
                    n_items += 1;
                }
                Err(ViewError::NoData) => unsafe {
                    bindings::crapolines::crapoline_show_confirmation(this.nbgl_page_content);
                },
                Err(_) => {
                    ui.show_error();
                }
            }
        }

        unsafe { bindings::crapolines::crapoline_show_items(this.nbgl_page_content, n_items) };
    }

    fn wait_ui(&mut self) {}

    fn expert(&self) -> bool {}

    fn toggle_expert(&mut self) {}

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
        let size = core::mem::size_of::<V>();
        unsafe {
            let apdu_buffer = apdu_buffer_mut();

            let buf_len = apdu_buffer.len();
            if size > buf_len {
                return None;
            }

            let new_loc_slice = &mut apdu_buffer[buf_len - size..];
            let new_loc_raw_ptr: *mut u8 = new_loc_slice.as_mut_ptr();
            let new_loc: *mut V = new_loc_raw_ptr.cast();

            //write but we don't want to drop `new_loc` since
            // it's not actually valid T data
            core::ptr::write(new_loc, viewable);

            //write how many bytes we have occupied
            self.viewable_size = size;

            //we can unwrap as we know this ptr is valid
            Some(new_loc.as_mut().unwrap().into())
        }
    }
}

mod cabi {
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn view_init_impl(msg: *mut u8) {
        //this exists to force Lazy initialization from the C side
        *IDLE_MESSAGE = msg;
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_expert_toggle() {
        RUST_ZUI.backend.toggle_expert();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_expert_update() {
        RUST_ZUI.backend.update_expert();
    }

    #[no_mangle]
    pub unsafe extern "C" fn view_idle_show_impl(item_idx: u8, status: *mut i8) {
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

    /// This needs to refresh the UI with the content for the given page
    ///
    /// Will be called by `nbgl_useCaseRegularView`
    #[no_mangle]
    pub unsafe extern "C" fn rs_transaction_screen(
        page: cty::uint8_t,
        nbgl_page_content: *mut cty::c_void,
    ) -> cty::c_uint {
        if let Err(e) = RUST_ZUI.skip_to_item(page as usize) {
            return e.into();
        }

        // store the nbgl page content pointer in the backend to pass it thru if necessary
        RUST_ZUI.backend.nbgl_page_content = nbgl_page_content.cast();
        StaxBackend::update_review(&mut RUST_ZUI);
        RUST_ZUI.backend.nbgl_page_content = core::ptr::null_mut();

        0
    }
}

mod bindings;

mod continuations {
    use super::RUST_ZUI;

    /// Continuation callback to kickoff the regular review flow
    pub fn review_transaction() {
        let total_pages = unsafe { RUST_ZUI.n_items() };

        bindings::use_case_regular_review(0, total_pages);
    }
}
