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
use super::UIBackend;
use crate::{
    ui::{apdu_buffer_mut, manual_vtable::RefMutDynViewable, store_into, Viewable},
    ui_toolkit::{strlen, Zui},
};
use bolos_derive::pic_str;
use bolos_sys::pic::PIC;

use arrayvec::ArrayString;

pub const KEY_SIZE: usize = 17 + 1;
//with null terminator
pub const MESSAGE_LINE_SIZE: usize = 17 + 1;
const MESSAGE_SIZE: usize = 2 * MESSAGE_LINE_SIZE - 1;

const INCLUDE_ACTIONS_AS_ITEMS: usize = 2;
const INCLUDE_ACTIONS_COUNT: usize = INCLUDE_ACTIONS_AS_ITEMS - 1;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: Zui<NanoSBackend, KEY_SIZE> = Zui::new();

#[bolos_derive::lazy_static(cbindgen)]
static mut BACKEND: NanoSBackend = NanoSBackend::default();

const DEFAULT_IDLE: &[u8] = b"DO NOT USE\x00";

#[bolos_derive::lazy_static]
pub static mut IDLE_MESSAGE: *const u8 = core::ptr::null();

#[repr(C)]
pub struct NanoSBackend {
    key: [u8; KEY_SIZE],
    value: [u8; MESSAGE_LINE_SIZE],
    value2: [u8; MESSAGE_LINE_SIZE],

    viewable_size: usize,
    expert: bool,
}

impl Default for NanoSBackend {
    fn default() -> Self {
        Self {
            key: [0; KEY_SIZE],
            value: [0; MESSAGE_LINE_SIZE],
            value2: [0; MESSAGE_LINE_SIZE],
            viewable_size: 0,
            expert: false,
        }
    }
}

impl UIBackend<KEY_SIZE> for NanoSBackend {
    type MessageBuf = ArrayString<MESSAGE_SIZE>;

    const INCLUDE_ACTIONS_COUNT: usize = INCLUDE_ACTIONS_COUNT;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        &mut self.key
    }

    fn message_buf(&mut self) -> Self::MessageBuf {
        let init = PIC::new(&[0; MESSAGE_SIZE]).into_inner();
        ArrayString::from_byte_string(init)
            .unwrap_or_else(|_| unsafe { core::hint::unreachable_unchecked() })
    }

    fn split_value_field(&mut self, message_buf: Self::MessageBuf) {
        //compute len and split `message_buf` at the max line size or at the total len
        // if the total len is less than the size of 1 line

        let len = strlen(message_buf.as_bytes()).unwrap_or_else(|_| message_buf.as_bytes().len());

        let (line1, line2) = if len >= MESSAGE_LINE_SIZE {
            //we need to split the buffer to fit in 2 lines
            // at LINE_SIZE - 1 since we need to allow line1 to have it's null terminator
            message_buf[..len].split_at(MESSAGE_LINE_SIZE - 1)
        } else {
            //no need to split the buffer, so line 2 will be empty
            (&message_buf[..len], pic_str!(""))
        };

        //write the 2 lines, so if the message was small enough to fit
        // on the first line
        // then the second line will stay empty
        self.value[..line1.len()].copy_from_slice(line1.as_bytes());
        self.value[line1.len()] = 0;

        self.value2[..line2.len()].copy_from_slice(&line2.as_bytes());
        self.value2[line2.len()] = 0; //make sure it's 0 terminated (line1 already is)
    }

    fn show_idle(&mut self, item_idx: usize, status: Option<&[u8]>) {
        let status = status
            .or_else(|| unsafe {
                PIC::new(*IDLE_MESSAGE).into_inner().as_ref().map(|status| {
                    let len = crate::ui_toolkit::c_strlen(status, KEY_SIZE).unwrap_or(KEY_SIZE);

                    core::slice::from_raw_parts(status, len)
                })
            })
            .unwrap_or_else(|| PIC::new(DEFAULT_IDLE).into_inner());

        let len = core::cmp::min(self.key.len() - 1, status.len());
        self.key[..len].copy_from_slice(&status[..len]);
        self.key[len] = 0; //0 terminate

        self.update_expert();

        unsafe {
            bindings::crapoline_ux_menu_display(item_idx as u8);
        }
    }

    fn show_error(&mut self) {
        unsafe {
            bindings::crapoline_ux_display_view_error();
        }
    }

    fn show_message(&mut self, title: &str, message: &str) {
        if let Ok(message) = ArrayString::from(message) {
            self.split_value_field(message);

            let title = title.as_bytes();

            let len = core::cmp::min(self.key.len(), title.len());
            self.key[..len].copy_from_slice(&title[..len]);
        }

        unsafe {
            bindings::crapoline_ux_display_view_message();
        }
    }

    fn show_review(ui: &mut Zui<Self, KEY_SIZE>) {
        //reset ui struct
        ui.paging_init();

        match ui.review_update_data() {
            Ok(_) => unsafe {
                bindings::crapoline_ux_display_view_review();
            },
            Err(_) => ui.show_error(),
        }
    }

    fn update_review(ui: &mut Zui<Self, KEY_SIZE>) {
        match ui.review_update_data() {
            Ok(_) => unsafe {
                bindings::crapoline_ux_display_view_review();
            },
            Err(_) => {
                ui.show_error();
                ui.backend.wait_ui();
            }
        }
    }

    fn wait_ui(&mut self) {
        unsafe {
            bindings::crapoline_ux_wait();
        }
    }

    fn expert(&self) -> bool {
        self.expert
    }

    fn toggle_expert(&mut self) {
        self.expert = !self.expert;

        self.show_idle(1, None);
    }

    fn update_expert(&mut self) {
        let msg = if self.expert {
            &pic_str!(b"enabled")[..]
        } else {
            &pic_str!(b"disabled")[..]
        };

        self.value[..msg.len()].copy_from_slice(msg);
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
    pub unsafe extern "C" fn rs_h_paging_can_decrease() -> bool {
        RUST_ZUI.paging_can_decrease()
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_paging_can_increase() -> bool {
        RUST_ZUI.paging_can_increase()
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
    pub unsafe extern "C" fn rs_h_error_accept(_: cty::c_uint) {
        RUST_ZUI.accept_error();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_both() {
        RUST_ZUI.review_action();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_left() {
        RUST_ZUI.left_button();
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_button_right() {
        RUST_ZUI.right_button();
    }
}

mod bindings {
    extern "C" {
        pub fn crapoline_ux_wait();
        pub fn crapoline_ux_menu_display(item_idx: u8);

        pub fn crapoline_ux_display_view_error();
        pub fn crapoline_ux_display_view_review();
        pub fn crapoline_ux_display_view_message();
    }
}
