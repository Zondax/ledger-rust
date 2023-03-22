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

const APDU_INDEX_CLA: usize = 0;
const APDU_INDEX_INS: usize = 1;
const APDU_INDEX_P1: usize = 2;
const APDU_INDEX_P2: usize = 3;
const APDU_INDEX_LEN: usize = 4;

const APDU_MIN_LENGTH: u32 = 5;

/// Wrap an "APDU Buffer" and provide accessor for different items
///
/// This helps avoiding accidental access at a wrong index for parameters and data,
/// whilst also exposing an API that prevents accidental reads after data has been written.
///
/// This is because the inner slice is not accessible for writing unless the entire structure is consumed,
/// doing so invalidates the references given out when reading slices of data (such as in [`payload`]).
///
/// Lastly, when the structure is consumed by way of [`write`], the inner buffer will be zeroed out.
pub struct ApduBufferRead<'apdu> {
    inner: &'apdu mut [u8],
}

#[cfg_attr(any(test, feature = "derive-debug"), derive(Debug))]
#[derive(PartialEq)]
pub enum ApduBufferReadError {
    /// The provided buffer was not long enough
    ///
    /// This happens if the slice is less than it should be for minimum
    /// or if the provided slice is too short for the provided `rx`
    LengthMismatch { expected: usize, got: usize },

    /// The requested payload was too short then expected
    NotEnoughPayload { expected: usize, got: usize },

    /// The provided buffer was too short and didn't have a payload
    NoPayload,
}

impl ApduBufferReadError {
    fn length_to_payload(self) -> Self {
        match self {
            Self::LengthMismatch { expected, got } => Self::NotEnoughPayload { expected, got },
            err => err,
        }
    }
}

impl<'apdu> ApduBufferRead<'apdu> {
    //make sure `len` is at least `expected`, given the `offset` into `len`
    // lierally len - offset < expected
    fn check_min_len(
        len: usize,
        expected: usize,
        offset: impl Into<Option<usize>>,
    ) -> Result<(), ApduBufferReadError> {
        let got = len.saturating_sub(offset.into().unwrap_or_default());
        if got < expected {
            Err(ApduBufferReadError::LengthMismatch { expected, got })
        } else {
            Ok(())
        }
    }

    /// Create a new "ApduBuffer" from the given mutable byte slice
    ///
    /// The function checks if there's at least the minimum required number of bytes (APDU_MIN_LENGTH)
    /// and if the byte slice is at least as long as rx
    pub fn new(buf: &'apdu mut [u8], rx: u32) -> Result<Self, ApduBufferReadError> {
        //check buf is at least 4
        Self::check_min_len(buf.len(), APDU_MIN_LENGTH as usize, None)?;

        //check rx is at least 4
        Self::check_min_len(rx as usize, APDU_MIN_LENGTH as usize, None)?;

        //check buf is at least rx
        Self::check_min_len(buf.len(), rx as usize, None)?;

        Ok(Self { inner: buf })
    }

    /// Alias to idx APDU_INDEX_CLA
    pub fn cla(&self) -> u8 {
        self.inner[APDU_INDEX_CLA]
    }

    /// Alias to idx APDU_INDEX_INS
    pub fn ins(&self) -> u8 {
        self.inner[APDU_INDEX_INS]
    }

    /// Alias to idx APDU_INDEX_P1
    pub fn p1(&self) -> u8 {
        self.inner[APDU_INDEX_P1]
    }

    /// Alias to idx APDU_INDEX_P2
    pub fn p2(&self) -> u8 {
        self.inner[APDU_INDEX_P2]
    }

    /// Return the remaining part of the buffer if present
    ///
    /// It's expected the buffer to have the prepended len at idx APDU_INDEX_LEN,
    /// thus the data would start at idx 5 until len - 5
    pub fn payload(&self) -> Result<&[u8], ApduBufferReadError> {
        let plen = self.inner[APDU_INDEX_LEN] as usize;
        //check that the buffer is long enough for the payload
        Self::check_min_len(self.inner.len(), plen as usize, APDU_MIN_LENGTH as usize)
            .map_err(|err| err.length_to_payload())?;

        Ok(&self.inner[APDU_MIN_LENGTH as usize..APDU_MIN_LENGTH as usize + plen])
        //we checked the size beforehand
    }

    /// Discard the structure to obtain the inner slice for writing
    pub fn write(self) -> &'apdu mut [u8] {
        zeroize::Zeroize::zeroize(self.inner);

        self.inner
    }
}
