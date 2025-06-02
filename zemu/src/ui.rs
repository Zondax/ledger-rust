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
use bolos_sys::raw::{io_exchange, CHANNEL_APDU, IO_ASYNCH_REPLY, IO_RETURN_AFTER_TX};

use bolos_sys::pic::PIC;

mod comm;
pub use comm::*;

/// cbindgen:ignore
pub(self) mod bindings {
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]

    include!("ui/bindings.rs");
}

pub(crate) mod manual_vtable;
use manual_vtable::RefMutDynViewable;

impl Into<bindings::zxerr_t> for ViewError {
    fn into(self) -> bindings::zxerr_t {
        match self {
            Self::Unknown | Self::Reject => bindings::zxerr_t_zxerr_unknown,
            Self::NoData => bindings::zxerr_t_zxerr_no_data,
        }
    }
}

pub(crate) fn apdu_buffer_mut() -> &'static mut [u8] {
    PIC::new(unsafe { &mut bolos_sys::raw::G_io_tx_buffer }).into_inner()
}

pub(crate) fn store_into<'buf, T: Sized>(
    item: T,
    buf: &'buf mut [u8],
) -> Option<(usize, &'buf mut T)> {
    let size = core::mem::size_of::<T>();
    let alignment = core::mem::align_of::<T>();

    unsafe {
        let buf_ptr = buf.as_mut_ptr();
        let buf_end = buf_ptr.add(buf.len());
        let new_loc_ptr = buf_end.sub(size);
        //becuase we are aligning backwards we need to calculate the opposite offset
        let new_loc_aligned = new_loc_ptr.sub(alignment - new_loc_ptr.align_offset(alignment));

        //actual size
        let size = new_loc_aligned.offset_from(buf_end).unsigned_abs();
        if size > buf.len() {
            //no space
            return None;
        }

        let new_loc: *mut T = new_loc_aligned.cast();

        //write but we don't want to drop `new_loc` since
        // it's not actually valid T data
        core::ptr::write(new_loc, item);

        //we can unwrap as we know this ptr is valid
        Some((size, new_loc.as_mut().unwrap()))
    }
}

impl<T: Viewable + Sized + 'static> Show for T {
    unsafe fn show(self, flags: &mut u32) -> Result<(usize, u16), ShowTooBig> {
        use crate::ui_toolkit::RUST_ZUI;

        RUST_ZUI.show(self)?;

        *flags |= IO_ASYNCH_REPLY;

        Ok((0, 0x9000))
    }
}
