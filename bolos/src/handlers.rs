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
pub mod prelude {
    pub use super::ApduHandler;
    pub use crate::{ApduBufferRead, ApduError};
}

use crate::{ApduBufferRead, ApduError};

/// Trait defining an APDU handler
pub trait ApduHandler {
    /// Entrypoint of the handler
    ///
    /// `flags` is used with the ui, to communicate to the system that some UI is runing
    /// `apdu_buffer` is the input (and output buffer).
    ///
    /// The return tells how many bytes of output were written, or an error code.
    fn handle(flags: &mut u32, apdu_buffer: ApduBufferRead) -> Result<u32, ApduError>;
}

/// Handler function signature
pub type HandlerFn = fn(&mut u32, ApduBufferRead) -> Result<u32, ApduError>;

/// Enum representing the different supported types of rules to choose which handler to run
pub enum HandlerRule {
    /// The handler will be selected when the ADPU's INSTRUCTION value matches the given one
    Instruction(u8),
    /// The handler will always be selected if it has this rule
    Always,
}

/// Structure representing the handlers accepted by [`apdu_dispatch`]
pub struct Handler {
    pub(crate) rule: HandlerRule,
    handler: crate::PIC<HandlerFn>,
}

impl Handler {
    /// Create a new [`Handler`]
    pub fn new(rule: HandlerRule, f: HandlerFn) -> Self {
        Self {
            rule,
            handler: crate::PIC::new(f),
        }
    }

    /// Shorthand to create a new simpler handler
    pub fn simple_handler<T: ApduHandler>(ins: u8) -> Self {
        Self::new(HandlerRule::Instruction(ins), T::handle)
    }

    fn handler(&self) -> &HandlerFn {
        self.handler.get_ref()
    }
}

#[inline(never)]
/// Dispatch the APDU based on the list of given `handlers`
///
/// The list is accessed as given, meaning earlier handlers take precedence.
pub fn apdu_dispatch<const CLA: u8>(
    flags: &mut u32,
    apdu_buffer: ApduBufferRead,
    handlers: &[Handler],
) -> Result<u32, ApduError> {
    *flags = 0;

    if apdu_buffer.cla() != CLA {
        return Err(ApduError::ClaNotSupported);
    }

    let ins = apdu_buffer.ins();

    //Search thru the registered instructions
    // and invoke the handler if found
    for hndl in handlers {
        let handler = match hndl.rule {
            HandlerRule::Instruction(hins) if hins == ins => hndl.handler(),
            HandlerRule::Always => hndl.handler(),
            _ => continue,
        };

        return handler(flags, apdu_buffer);
    }

    Err(ApduError::CommandNotAllowed)
}
