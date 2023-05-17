//! This module contains definitions and wrappers
//! for C functions used in the Stax UI

use self::crapolines::nbgl::NbglCallback;

pub mod crapolines {
    extern "C" {
        pub fn crapoline_ux_wait();
        pub fn crapoline_ux_flow_init_idle_flow_toggle_expert();
        pub fn crapoline_ux_show_review();
        pub fn crapoline_ux_show_error();

        pub fn crapoline_home();

        pub fn crapoline_show_confirmation(nbgl_page_content: *mut ());
        pub fn crapoline_show_items(nbgl_page_ontent: *mut (), nbPairs: cty::uint8_t);
    }

    ///NBGL crapolines
    pub mod nbgl {
        pub type NbglCallback = unsafe extern "C" fn();

        extern "C" {
            /// Generic callback
            //this will pass the icon for us
            // and the (fixed) rejection text
            pub fn crapoline_useCaseReviewStart(
                title: *mut cty::c_char,
                subtitle: *mut cty::c_char,
                continuation: NbglCallback,
                reject: NbglCallback,
            );

            // this will pass the (fixed) rejection text
            // along with the nav callback and the choice callback
            // TODO: choice callback should have a confirmation screen
            pub fn crapoline_useCaseRegularReview(
                first_page: cty::uint8_t,
                n_total_pages: cty::uint8_t,
            );

            // this will pass the (fixed) rejection text
            // along with the update callback and choice callback
            // TODO: choice callback should have a confirmation screen
            pub fn crapoline_useCaseStaticReview(n_total_pages: cty::uint8_t);
        }

        pub use crapoline_useCaseRegularReview as use_case_regular_review;
        pub use crapoline_useCaseReviewStart as use_case_review_start;
        pub use crapoline_useCaseStaticReview as use_case_static_review;
    }
}

fn pic_callback(cb: NbglCallback) -> NbglCallback {
    let to_pic = cb as usize;
    let picced = unsafe { bolos_sys::pic::PIC::manual(to_pic) };

    unsafe { core::mem::transmute(picced) }
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

    //I'd like to reuse rs_h_reject but alas
    // (u32) and () are different
    unsafe extern "C" fn reject() {
        super::RUST_ZUI.reject();
    }

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
pub fn use_case_regular_review(first_page: u8, n_total_pages: u8) {
    unsafe { crapolines::nbgl::use_case_regular_review(first_page, n_total_pages) }
}

#[inline(always)]
pub fn use_case_static_review(n_total_pages: u8) {
    unsafe { crapolines::nbgl::use_case_static_review(n_total_pages) }
}
