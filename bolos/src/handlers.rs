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
pub use linkme::distributed_slice as register;

pub mod prelude {
    pub use super::{register, ApduHandler, HandlerFn, HANDLERS};
    pub use crate::{ApduBufferRead, ApduError};
}

use crate::{ApduBufferRead, ApduError};

/// Trait defining an APDU handler
pub trait ApduHandler {
    /// Entrypoint of the handler
    ///
    /// `flags` is used with the ui, to communicate to the system that some UI is runing
    /// `apdu_buffer` is the input (and output buffer
    ///
    /// The return tells how many bytes of output were written, or an error code.
    fn handle(flags: &mut u32, apdu_buffer: ApduBufferRead) -> Result<u32, ApduError>;
}

/// Handler function signature
pub type HandlerFn = fn(&mut u32, ApduBufferRead) -> Result<u32, ApduError>;

#[register]
pub static HANDLERS: [(u8, HandlerFn)] = [..];

#[inline(never)]
pub fn apdu_dispatch<const CLA: u8>(
    flags: &mut u32,
    apdu_buffer: ApduBufferRead,
    handlers: &[(u8, HandlerFn)],
) -> Result<u32, ApduError> {
    *flags = 0;

    if apdu_buffer.cla() != CLA {
        return Err(ApduError::ClaNotSupported);
    }

    let ins = apdu_buffer.ins();

    //Search thru the registered instructions
    // and invoke the handler if found
    for (hins, handler) in handlers {
        if *hins == ins {
            let handler = crate::PIC::new(handler).into_inner();
            return handler(flags, apdu_buffer);
        }
    }

    Err(ApduError::CommandNotAllowed)
}
