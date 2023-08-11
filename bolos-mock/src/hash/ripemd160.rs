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
use core::mem::MaybeUninit;
use ripemd::digest::{Digest, FixedOutputReset};
use std::convert::Infallible;

pub struct Ripemd160(ripemd::Ripemd160);

impl Ripemd160 {
    pub const DIGEST_LEN: usize = 20;

    pub fn new() -> Result<Self, Infallible> {
        Ok(Self(ripemd::Ripemd160::new()))
    }

    pub fn new_gce(loc: &mut MaybeUninit<Self>) -> Result<(), Infallible> {
        *loc = MaybeUninit::new(Self::new()?);

        Ok(())
    }
}

/*
 * pub trait Hasher<const S: usize> {
        type Error;

        /// Add data to hasher
        fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

        /// Consume hasher and retrieve output
        fn finalize(mut self) -> Result<[u8; S], Self::Error>;

        /// One-short digest
        fn digest(input: &[u8]) -> Result<[u8; S], Error>;
    }
*/
impl super::Hasher<{ Ripemd160::DIGEST_LEN }> for Ripemd160 {
    type Error = Infallible;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize_dirty_into(&mut self, out: &mut [u8; Self::DIGEST_LEN]) -> Result<(), Self::Error> {
        let digest = self.0.finalize_fixed_reset();
        out.copy_from_slice(digest.as_ref());

        Ok(())
    }

    fn finalize_into(self, out: &mut [u8; Self::DIGEST_LEN]) -> Result<(), Self::Error> {
        let digest = self.0.finalize();
        out.copy_from_slice(digest.as_ref());

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.0.reset();
        Ok(())
    }

    fn digest_into(input: &[u8], out: &mut [u8; Self::DIGEST_LEN]) -> Result<(), Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize_into(out)
    }
}

impl super::HasherId for Ripemd160 {
    type Id = u8;

    fn id() -> Self::Id {
        4
    }
}
