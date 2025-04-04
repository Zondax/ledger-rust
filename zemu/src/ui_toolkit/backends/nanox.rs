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
    ui::{apdu_buffer_mut, manual_vtable::RefMutDynViewable, store_into, ViewError, Viewable},
    ui_toolkit::Zui,
};
use bolos_derive::pic_str;
use bolos_sys::pic::PIC;

pub const KEY_SIZE: usize = 63 + 1;
//with null terminator
pub const MESSAGE_SIZE: usize = 4095 + 1;

const INCLUDE_ACTIONS_COUNT: usize = 0;

#[bolos_derive::lazy_static]
pub static mut RUST_ZUI: Zui<NanoXBackend, KEY_SIZE> = Zui::new();

#[bolos_derive::lazy_static(cbindgen)]
static mut BACKEND: NanoXBackend = NanoXBackend::default();

const DEFAULT_IDLE: &[u8] = b"DO NOT USE\x00";

#[bolos_derive::lazy_static]
pub static mut IDLE_MESSAGE: *const u8 = core::ptr::null();

#[repr(C)]
pub struct NanoXBackend {
    key: [u8; KEY_SIZE],
    message: [u8; MESSAGE_SIZE],

    viewable_size: usize,
    expert: bool,

    flow_inside_loop: bool,
}

impl Default for NanoXBackend {
    fn default() -> Self {
        Self {
            key: [0; KEY_SIZE],
            message: [0; MESSAGE_SIZE],
            viewable_size: 0,
            expert: false,
            flow_inside_loop: false,
        }
    }
}

impl NanoXBackend {
    pub fn review_loop_start(&mut self, ui: &mut Zui<Self, KEY_SIZE>) {
        if self.flow_inside_loop {
            //coming from right

            if !ui.paging_can_decrease() {
                //exit to the left
                self.flow_inside_loop = false;
                unsafe {
                    bindings::crapoline_ux_flow_prev();
                }

                return;
            }

            ui.paging_decrease();
        } else {
            ui.paging_init();
        }

        Self::update_review(ui);

        unsafe {
            bindings::crapoline_ux_flow_next();
        }
    }

    pub fn review_loop_end(&mut self, ui: &mut Zui<Self, KEY_SIZE>) {
        if self.flow_inside_loop {
            //coming from left
            ui.paging_increase();

            match ui.review_update_data() {
                Ok(_) => unsafe {
                    bindings::crapoline_ux_layout_bnnn_paging_reset();
                },
                Err(ViewError::NoData) => {
                    self.flow_inside_loop = false;
                    unsafe {
                        bindings::crapoline_ux_flow_next();
                    }
                    return;
                }
                Err(_) => ui.show_error(),
            }
        } else {
            ui.paging_decrease();
            Self::update_review(ui);
        }

        unsafe {
            bindings::crapoline_ux_flow_relayout();
        }
    }
}

impl UIBackend<KEY_SIZE> for NanoXBackend {
    type MessageBuf = &'static mut str;

    const INCLUDE_ACTIONS_COUNT: usize = 0;

    fn static_mut() -> &'static mut Self {
        unsafe { &mut BACKEND }
    }

    fn update_expert(&mut self) {
        let msg = if self.expert {
            &pic_str!(b"enabled")[..]
        } else {
            &pic_str!(b"disabled")[..]
        };

        self.message[..msg.len()].copy_from_slice(msg);
    }

    fn key_buf(&mut self) -> &mut [u8; KEY_SIZE] {
        &mut self.key
    }

    fn message_buf(&mut self) -> Self::MessageBuf {
        // We do not know what is self, but the compiler
        // does, so let it know we are done with self
        // at this point
        _ = self;

        core::str::from_utf8_mut(&mut Self::static_mut().message)
            //this should never happen as we always asciify
            .unwrap_or_else(|_| unsafe { core::hint::unreachable_unchecked() })
    }

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

        //truncate status
        let len = core::cmp::min(self.key.len() - 1, status.len());
        self.key[..len].copy_from_slice(status);
        self.key[len] = 0; //0 terminate

        unsafe {
            bindings::crapoline_ux_show_idle();
        }
    }

    fn show_error(&mut self) {
        unsafe {
            bindings::crapoline_ux_show_error();
        }
    }

    fn show_message(&mut self, _title: &str, _message: &str) {
        panic!("capability not supported on nanox yet?")
    }

    fn show_review(ui: &mut Zui<Self, KEY_SIZE>) {
        //reset ui struct
        ui.paging_init();

        unsafe {
            //we access the backend directly here instead
            // of going thru RUST_ZUI since otherwise we don't have access
            // to this functionality
            BACKEND.flow_inside_loop = false;

            bindings::crapoline_ux_show_review();
        }
    }

    fn update_review(ui: &mut Zui<Self, KEY_SIZE>) {
        match ui.review_update_data() {
            Ok(_) | Err(ViewError::NoData) => {}
            Err(_) => {
                ui.show_error();
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

        unsafe {
            bindings::crapoline_ux_flow_init_idle_flow_toggle_expert();
        }
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

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_start() {
        BACKEND.review_loop_start(&mut RUST_ZUI)
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_inside() {
        BACKEND.flow_inside_loop = true;
    }

    #[no_mangle]
    pub unsafe extern "C" fn rs_h_review_loop_end() {
        BACKEND.review_loop_end(&mut RUST_ZUI)
    }
}

mod bindings {
    extern "C" {
        pub fn crapoline_ux_wait();
        pub fn crapoline_ux_flow_init_idle_flow_toggle_expert();
        pub fn crapoline_ux_show_review();
        pub fn crapoline_ux_show_error();
        pub fn crapoline_ux_show_idle();
        pub fn crapoline_ux_flow_prev();
        pub fn crapoline_ux_flow_next();
        pub fn crapoline_ux_layout_bnnn_paging_reset();
        pub fn crapoline_ux_flow_relayout();
    }
}
