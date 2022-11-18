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
mod packet;
use packet::PacketType;

use crate::{
    lock::LockError, nvm::NVMError, pic::PIC, ApduBufferRead, ApduError, Lock, SwappingBuffer,
};

#[bolos::lazy_static]
//this is okay to not lock as we access this
// only if we can lock the buffer
//
// also, declaring a lock here to match
// the given buffer would be rather problematic to do
static mut INIT_LEN: usize = 0;

/// Upload protocol implementation
///
/// Easy to use and ergonomic way to add support for multi-message payload upload.
/// Requires a backing [`SwappingBuffer`] to store the data being uploaded.
///
/// # Example
/**
```rust
# use bolos::{uploader::Uploader, Lock, SwappingBuffer, ApduBufferRead};
# fn example<'m, A: Eq + Copy, const X: usize, const Y: usize>(input: ApduBufferRead<'_>, buffer: &mut Lock<SwappingBuffer<'m, 'm, X, Y>, A>, accessor: A) {
Uploader::new(accessor, buffer).upload(&input);
# }
```
**/
pub struct Uploader<'buf, 'm, A, const X: usize, const Y: usize> {
    accessor: A,
    //we need to have a separate lifetime for the inner buffer
    // to allow the reference to Lock be different than the one in the Buffer
    // to prevent locking the refenrece to an unnecessary lifetime
    // (causing issues when instantiating the uploader)
    buffer: &'buf mut Lock<SwappingBuffer<'m, 'm, X, Y>, A>,
}

#[cfg_attr(any(test, feature = "derive-debug"), derive(Debug))]
pub enum UploaderError {
    /// Couldn't parse PacketType
    PacketTypeParseError,

    /// PacketType wasn't init, next or last
    PacketTypeInvalid,

    /// Error with the buffer's lock
    Lock(LockError),

    /// Error writing to buffer
    Nvm(NVMError),
}

impl From<LockError> for UploaderError {
    fn from(e: LockError) -> Self {
        Self::Lock(e)
    }
}

impl From<NVMError> for UploaderError {
    fn from(e: NVMError) -> Self {
        Self::Nvm(e)
    }
}

impl From<UploaderError> for ApduError {
    fn from(e: UploaderError) -> Self {
        match e {
            UploaderError::PacketTypeInvalid | UploaderError::PacketTypeParseError => {
                ApduError::InvalidP1P2
            }
            UploaderError::Nvm(_) => ApduError::DataInvalid,
            UploaderError::Lock(e) => match e {
                LockError::NotLocked => Self::ExecutionError,
                LockError::Busy | LockError::BadId => Self::Busy,
            },
        }
    }
}

pub struct UploaderOutput<'m> {
    pub p2: u8,
    pub first: &'m [u8],
    pub data: &'m [u8],
}

impl<'buf, 'm, A: Eq + Copy, const X: usize, const Y: usize> Uploader<'buf, 'm, A, X, Y> {
    /// Instantiate a new [`Uploader`] using the given `accessor` to manage access to `buffer`
    pub fn new(
        accessor: impl Into<A>,
        buffer: &'buf mut Lock<SwappingBuffer<'m, 'm, X, Y>, A>,
    ) -> Self {
        Self {
            accessor: accessor.into(),
            buffer,
        }
    }

    #[inline(never)]
    /// Consume the given `input` message and return the entire payload if the upload is complete
    ///
    /// Will return errors of the backing buffer or if the flow of operations is incorrect
    pub fn upload(
        self,
        input: &ApduBufferRead<'_>,
    ) -> Result<Option<UploaderOutput<'buf>>, UploaderError> {
        let packet_type =
            PacketType::new(input.p1()).map_err(|_| UploaderError::PacketTypeParseError)?;

        if packet_type.is_init() {
            let zbuffer = self.buffer.lock(self.accessor)?;
            zbuffer.reset();

            zbuffer.write(&[input.p2()])?;
            if let Ok(payload) = input.payload() {
                unsafe {
                    *INIT_LEN = payload.len();
                }
                zbuffer.write(payload)?;
            }

            Ok(None)
        } else if packet_type.is_next() {
            let zbuffer = self.buffer.acquire(self.accessor)?;

            if let Ok(payload) = input.payload() {
                zbuffer.write(payload)?;
            }

            Ok(None)
        } else if packet_type.is_last() {
            let zbuffer = Lock::acquire(self.buffer, self.accessor)?;

            if let Ok(payload) = input.payload() {
                zbuffer.write(payload)?;
            }

            let data = zbuffer.read_exact_and_reset();
            let (head, tail) = data[1..].split_at(unsafe { std::mem::replace(&mut *INIT_LEN, 0) });

            Ok(Some(UploaderOutput {
                p2: data[0],
                first: head,
                data: tail,
            }))
        } else {
            Err(UploaderError::PacketTypeInvalid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::*;

    const RAM_SIZE: usize = 9; //1(p2) + msg
    const FLASH_SIZE: usize = 0;

    fn setup() -> Lock<SwappingBuffer<'static, 'static, RAM_SIZE, FLASH_SIZE>, bool> {
        let buffer = new_swapping_buffer!(RAM_SIZE, FLASH_SIZE);

        Lock::new(buffer)
    }

    fn payload_chunks<const MAX: usize>(p2: u8, payload: &[u8]) -> Vec<Vec<u8>> {
        //force first (init) packet
        let mut v: Vec<Vec<u8>> = std::iter::once(vec![0xFF, 0xFF, PacketType::Init as u8, p2, 0])
            //upload data chunk by chunk
            .chain(payload.chunks(MAX).map(|chunk| {
                let mut vv = Vec::with_capacity(5 + chunk.len());
                vv.extend_from_slice(&[0xFF, 0xFF, PacketType::Add as u8, p2, chunk.len() as u8]);
                vv.extend_from_slice(chunk);
                vv
            }))
            .collect();

        //mark last packet as Last
        v.last_mut().unwrap()[2] = PacketType::Last as u8;

        v
    }

    #[test]
    fn uploader() {
        let mut buffer = setup();
        let data = payload_chunks::<2>(0, b"deadbeef").into_iter();

        for mut chunk in data {
            let len = chunk.len() as u32;
            println!("{:x?}", &chunk);
            let input = ApduBufferRead::new(chunk.as_mut_slice(), len).unwrap();

            if let Some(UploaderOutput { p2, first, data }) = Uploader::new(true, &mut buffer)
                .upload(&input)
                .expect("able to upload")
            {
                assert_eq!(p2, 0);
                assert_eq!(first, &[]);
                assert_eq!(data, b"deadbeef");
            }
        }
    }
}
