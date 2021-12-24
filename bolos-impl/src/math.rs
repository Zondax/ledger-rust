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
        if #[cfg(nanox)] {
            let might_throw = || unsafe {
                crate::raw::cx_math_cmp(a, b, len as _)
            };

            diff = catch(might_throw)?;
        } else if #[cfg(nanos)] {
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
        if #[cfg(nanox)] {
            let might_throw = || unsafe {
                crate::raw::cx_math_modm(v, v_len as _, m, m_len as _);
            };

            catch(might_throw)?;
            Ok(())
        } else if #[cfg(nanos)] {
            match unsafe { crate::raw::cx_math_modm_no_throw(v, v_len as _, m, m_len as _) } {
                0 => Ok(()),
                err => Err(err.into())
            }
        } else {
            unimplemented!("cx_math_modm called in non-bolos");
        }
    }
}
