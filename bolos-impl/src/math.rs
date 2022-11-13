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
#[allow(unused_imports)]
use crate::errors::{catch, Error};

use core::cmp::Ordering;

/// Compare two operands
pub fn cmp(a: &[u8], b: &[u8]) -> Result<Ordering, Error> {
    let len = core::cmp::min(a.len(), b.len());
    let a = a.as_ptr();
    let b = b.as_ptr();

    let mut diff = 0;

    cfg_if! {
        if #[cfg(bolos_sdk)] {
            match unsafe { crate::raw::cx_math_cmp_no_throw(a, b, len as _, &mut diff) } {
                0 => {},
                err => return Err(err.into())
            }
        } else {
            unimplemented!("cx_math_cmp called in non-bolos");
        }
    }

    if diff == 0 {
        Ok(Ordering::Equal)
    } else if diff < 0 {
        Ok(Ordering::Less)
    } else {
        Ok(Ordering::Greater)
    }
}

/// Modulo operation
///
/// Applies v % m storing the result in v
pub fn modm(v: &mut [u8], m: &[u8]) -> Result<(), Error> {
    let (v, v_len) = (v.as_mut_ptr(), v.len());
    let (m, m_len) = (m.as_ptr(), m.len());

    cfg_if! {
        if #[cfg(bolos_sdk)] {
            match unsafe { crate::raw::cx_math_modm_no_throw(v, v_len as _, m, m_len as _) } {
                0 => Ok(()),
                err => Err(err.into())
            }
        } else {
            unimplemented!("cx_math_modm called in non-bolos");
        }
    }
}
