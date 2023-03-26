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
        extern "C" {
            /// Generic callback
            pub type NbglCallback = unsafe extern "C" fn();

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
        }

        pub use crapoline_useCaseRegularReview as use_case_regular_review;
        pub use crapoline_useCaseReviewStart as use_case_review_start;
    }
}

#[inline(always)]
pub fn use_case_review_start(title: &str, subtitle: Option<&str>, continuation: fn()) {
    let title = title.as_ptr().cast_mut() as *mut _;
    let subtitle = subtitle
        .map(|s| s.as_ptr())
        .unwrap_or(core::ptr::null())
        .as_mut() as *mut _;

    unsafe {
        crapolines::nbgl::use_case_review_start(
            title,
            subtitle,
            continuation as NbglCallback,
            super::cabi::rs_h_reject as NbglCallback,
        )
    }
}

#[inline(always)]
pub fn use_case_regular_review(first_page: u8, n_total_pages: u8) {
    unsafe { crapolines::nbgl::use_case_regular_review(first_page, n_total_pages) }
}
