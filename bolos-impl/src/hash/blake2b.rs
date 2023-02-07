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
#![allow(unused_imports)]

use crate::raw::{cx_blake2b_t, cx_hash_t, cx_md_t};
use crate::{errors::catch, Error};

use super::CxHash;

use core::{mem::MaybeUninit, ptr::addr_of_mut};

#[repr(transparent)]
pub struct Blake2b<const S: usize> {
    state: cx_blake2b_t,
}

impl<const S: usize> Blake2b<S> {
    pub const DIGEST_LEN: usize = S;

    #[inline(never)]
    pub fn new() -> Result<Self, Error> {
        zemu_sys::zemu_log_stack("Blake2b::new\x00");
        let mut this = Self {
            state: Default::default(),
        };

        Self::init_state_with_params(&mut this.state, &[], &[])?;

        Ok(this)
    }

    /// Initialize a new Blake2B hasher with default key and personalization
    pub fn new_gce(loc: &mut MaybeUninit<Self>) -> Result<(), Error> {
        let state = unsafe { addr_of_mut!((*loc.as_mut_ptr()).state) };

        Self::init_state_with_params(state, &[], &[])
    }

    /// Initialize a new Blake2B hasher with the given key and personalization
    ///
    /// Empty slices is the same as default initialization
    pub fn new_gce_with_params(
        loc: &mut MaybeUninit<Self>,
        salt: &[u8],
        personalization: &[u8],
    ) -> Result<(), Error> {
        let state = unsafe { addr_of_mut!((*loc.as_mut_ptr()).state) };

        Self::init_state_with_params(state, salt, personalization)
    }

    /// Initialize with the given params
    ///
    /// Using empty slices is equivalent to a default initialization
    fn init_state_with_params(
        state: *mut cx_blake2b_t,
        salt: &[u8],
        personalization: &[u8],
    ) -> Result<(), Error> {
        let salt = if salt.is_empty() {
            (std::ptr::null_mut(), 0)
        } else {
            (salt.as_ptr() as *mut u8, salt.len())
        };

        let perso = if personalization.is_empty() {
            (std::ptr::null_mut(), 0)
        } else {
            (personalization.as_ptr() as *mut u8, personalization.len())
        };

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                let r = unsafe {
                    crate::raw::cx_blake2b_init2_no_throw(
                        state,
                        S * 8,
                        salt.0,
                        salt.1,
                        perso.0,
                        perso.1
                    )
                };

                match r {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("blake2b init called in non bolos")
            }
        }

        Ok(())
    }
}

impl<const S: usize> CxHash<S> for Blake2b<S> {
    fn cx_init_hasher() -> Result<Self, Error> {
        Self::new()
    }

    fn cx_init_hasher_gce(loc: &mut MaybeUninit<Self>) -> Result<(), super::Error> {
        Self::new_gce(loc)
    }

    fn cx_reset(&mut self) -> Result<(), Error> {
        Self::init_state_with_params(&mut self.state, &[], &[])
    }

    fn cx_header(&mut self) -> &mut cx_hash_t {
        &mut self.state.header
    }

    fn cx_id() -> cx_md_t {
        crate::raw::cx_md_e_CX_BLAKE2B
    }
}
