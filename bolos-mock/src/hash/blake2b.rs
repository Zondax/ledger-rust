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
use blake2::digest::{Reset, Update, VariableOutputDirty};
use core::mem::MaybeUninit;

pub struct Blake2b<const S: usize>(blake2::VarBlake2b);

impl<const S: usize> Blake2b<S> {
    pub const DIGEST_LEN: usize = S;

    fn init_blake2b(salt: &[u8], personalization: &[u8]) -> blake2::VarBlake2b {
        blake2::VarBlake2b::with_params(&[], salt, personalization, S)
    }

    pub fn new_gce(loc: &mut MaybeUninit<Self>) -> Result<(), crate::Error> {
        *loc = MaybeUninit::new(Self::new()?);

        Ok(())
    }

    /// Initialize a new Blake2B hasher with the given key and personalization
    ///
    /// Empty slices is the same as default initialization
    pub fn new_gce_with_params(
        loc: &mut MaybeUninit<Self>,
        salt: &[u8],
        personalization: &[u8],
    ) -> Result<(), crate::Error> {
        *loc = MaybeUninit::new(Self(Self::init_blake2b(salt, personalization)));

        Ok(())
    }

    pub fn new() -> Result<Self, crate::Error> {
        Ok(Self(Self::init_blake2b(&[], &[])))
    }
}

impl<const S: usize> super::Hasher<S> for Blake2b<S> {
    type Error = crate::Error;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize_dirty_into(&mut self, out: &mut [u8; S]) -> Result<(), Self::Error> {
        self.0
            .finalize_variable_dirty(|digest| out.copy_from_slice(digest));

        Ok(())
    }

    fn finalize_into(mut self, out: &mut [u8; S]) -> Result<(), Self::Error> {
        self.0
            .finalize_variable_dirty(|digest| out.copy_from_slice(digest));

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        self.0.reset();
        Ok(())
    }

    fn digest_into(input: &[u8], out: &mut [u8; S]) -> Result<(), Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize_into(out)
    }
}

impl<const S: usize> super::HasherId for Blake2b<S> {
    type Id = u8;

    fn id() -> Self::Id {
        9
    }
}
