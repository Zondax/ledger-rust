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
