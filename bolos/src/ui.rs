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

use zemu_sys::ViewError;

use crate::LedgerUnwrap;

#[macro_export]
/// Convert the return of Show::show into something more usable for apdu handlers
///
/// sets `tx` to the amount returned if given,
/// otherwise tx is returned only on success and discarded on failure
macro_rules! show_ui {
    ($show:expr, $tx:ident) => {
        match unsafe { $show } {
            Ok((size, err)) if err == crate::ApduError::Success as u16 => {
                *$tx = size as _;
                Ok(())
            }
            Ok((size, err)) => {
                use ::core::convert::TryInto;
                *$tx = size as _;

                match err.try_into() {
                    Ok(err) => Err(err),
                    Err(_) => Err(crate::ApduError::ExecutionError),
                }
            }
            Err(_) => Err(crate::ApduError::ExecutionError),
        }
    };
    ($show:expr) => {
        match unsafe { $show } {
            Ok((size, err)) if err == crate::ApduError::Success as u16 => Ok(size as _),
            Ok((_, err)) => {
                use ::core::convert::TryInto;

                match err.try_into() {
                    Ok(err) => Err(err),
                    Err(_) => Err(crate::ApduError::ExecutionError),
                }
            }
            Err(_) => Err(crate::ApduError::ExecutionError),
        }
    };
}

///This trait defines the interface useful in the UI context
/// so that all the different OperationTypes or other items can handle their own UI
pub trait DisplayableItem {
    /// Returns the number of items to display
    fn num_items(&self) -> usize;

    /// This is invoked when a given page is to be displayed
    ///
    /// `item_n` is the item of the operation to display;
    /// guarantee: 0 <= item_n < self.num_items()
    /// `title` is the title of the item
    /// `message` is the contents of the item
    /// `page` is what page we are supposed to display, this is used to split big messages
    ///
    /// returns the total number of pages on success
    ///
    /// It's a good idea to always put `#[inline(never)]` on top of this
    /// function's implementation
    //#[inline(never)]
    fn render_item(
        &self,
        item_n: u8,
        title: &mut [u8],
        message: &mut [u8],
        page: u8,
    ) -> Result<u8, ViewError>;
}


#[inline(never)]
/// Perform paging of `input` into `out`
///
/// Will page based on the total length of the input,
/// the length of the output
/// and the given page number.
///
/// If `None` is returned an error occured, otherwise the total number of pages is returned
pub fn handle_message(input: &[u8], out: &mut [u8], page: u8) -> Option<u8> {
    let m_len = out.len() - 1; //null byte terminator
    if m_len <= input.len() {
        let chunk = input
            .chunks(m_len) //divide in non-overlapping chunks
            .nth(page as usize)?; //get the nth chunk

        //guaranteed to be less than out size
        out.get_mut(..chunk.len())
            .ledger_unwrap()
            .copy_from_slice(chunk);
        *out.get_mut(chunk.len()).ledger_unwrap() = 0; //null terminate

        let n_pages = input.len() / m_len;
        Some(n_pages as u8)
    } else {
        //guaranteed to be smaller than out
        out.get_mut(..input.len())
            .ledger_unwrap()
            .copy_from_slice(input);
        *out.get_mut(input.len()).ledger_unwrap() = 0; //null terminate
        Some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_input() {
        const MSG: &[u8] = b"deadbeef";

        let mut out = [42u8; 5];

        let pages = handle_message(&MSG, &mut out, 0).unwrap();
        assert_eq!(pages, 2);

        assert_eq!(&out[..4], &MSG[..4]);
        assert_eq!(out[4], 0);

        let pages = handle_message(&MSG, &mut out, 1).unwrap();
        assert_eq!(pages, 2);

        assert_eq!(&out[..4], &MSG[4..]);
        assert_eq!(out[4], 0);

        assert!(handle_message(&MSG, &mut out, 2).is_none());
    }

    #[test]
    fn single_page_input() {
        const MSG: &[u8] = b"deadbeef";

        let mut out = [42u8; 10];

        let pages = handle_message(&MSG, &mut out, 0).unwrap();
        assert_eq!(pages, 1);

        assert_eq!(&out[..8], &MSG[..]);
        assert_eq!(out[8], 0);

        // rest untouched
        assert_eq!(out[9], 42);

        // always gonna write the entire message, no matter the page
        // (since it fits in one page)
        assert!(handle_message(&MSG, &mut out, 1).is_some());
    }
}
