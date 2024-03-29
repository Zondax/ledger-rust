//! This module contains definitions and wrappers
//! for C functions used in the Stax UI

use self::crapolines::nbgl::NbglCallback;

pub mod crapolines {
    extern "C" {
        pub fn crapoline_home();
        pub fn crapoline_error();
        pub fn crapoline_message();
    }

    ///NBGL crapolines
    pub mod nbgl {
        pub type NbglCallback = unsafe extern "C" fn();

        extern "C" {
            /// Generic callback
            //this will pass the icon for us
            // and the (fixed) rejection text
            pub fn crapoline_useCaseReviewStart(
                title: *const cty::c_char,
                subtitle: *const cty::c_char,
                continuation: NbglCallback,
                reject: NbglCallback,
            );

            // this will pass the (fixed) rejection text
            // along with the update callback and choice callback
            // TODO: choice callback should have a confirmation screen
            pub fn crapoline_useCaseStaticReview(n_total_pages: cty::uint8_t);

            // this will pass the choice callback
            // and prepare the extra pages pointer if necessary
            // TODO: choice callback should have a confirmation screen
            pub fn crapoline_useCaseAddressConfirmationExt(n_total_pages: cty::uint8_t);

            pub fn crapoline_useCaseStatus(
                message: *const cty::c_char,
                is_success: bool,
                continuation: NbglCallback,
            );

        }

        pub use crapoline_useCaseAddressConfirmationExt as use_case_address_confirmation_ext;
        pub use crapoline_useCaseReviewStart as use_case_review_start;
        pub use crapoline_useCaseStaticReview as use_case_static_review;
        pub use crapoline_useCaseStatus as use_case_status;
    }
}

fn pic_callback(cb: NbglCallback) -> NbglCallback {
    let to_pic = cb as usize;
    let picced = unsafe { bolos_sys::pic::PIC::manual(to_pic) };

    unsafe { core::mem::transmute(picced) }
}

//I'd like to reuse rs_h_reject but alas
// (u32) and () are different
unsafe extern "C" fn reject() {
    super::RUST_ZUI.reject();
}

//I'd like to reuse rs_h_accept but alas
// (u32) and () are different
unsafe extern "C" fn accept() {
    super::RUST_ZUI.approve();
}

#[inline(always)]
pub fn use_case_review_start(
    title: &str,
    subtitle: Option<&str>,
    continuation: unsafe extern "C" fn(),
) {
    let title = title.as_ptr().cast_mut() as *mut _;
    let subtitle = subtitle
        .map(|s| s.as_ptr())
        .unwrap_or(core::ptr::null())
        .cast_mut() as *mut _;

    unsafe {
        crapolines::nbgl::use_case_review_start(
            title,
            subtitle,
            pic_callback(continuation as NbglCallback),
            pic_callback(reject as NbglCallback),
        )
    }
}

#[inline(always)]
pub fn use_case_static_review(n_total_pages: u8) {
    unsafe { crapolines::nbgl::use_case_static_review(n_total_pages) }
}

#[inline(always)]
pub fn use_case_address(n_total_pages: u8) {
    unsafe { crapolines::nbgl::use_case_address_confirmation_ext(n_total_pages) }
}

#[inline(always)]
pub fn use_case_status(message: &str, is_success: bool) {
    let message = if message.is_empty() {
        core::ptr::null()
    } else {
        message.as_ptr().cast_mut() as *mut _
    };
    let continuation = if is_success { accept } else { reject };

    unsafe {
        crapolines::nbgl::use_case_status(
            message,
            is_success,
            pic_callback(continuation as NbglCallback),
        )
    }
}
