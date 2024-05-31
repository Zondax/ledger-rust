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
use tiny_keccak::Hasher;

pub struct Keccak<const S: usize>(tiny_keccak::Keccak);

//This below is a workaround to allow to implement the functionality
// only for specific values of S

pub trait Keccak_<const S: usize>: Sized {
    fn new() -> Result<Self, crate::Error>;
}

impl Keccak_<28> for Keccak<28> {
    fn new() -> Result<Self, crate::Error> {
        Ok(Self(tiny_keccak::Keccak::v224()))
    }
}

impl Keccak_<32> for Keccak<32> {
    fn new() -> Result<Self, crate::Error> {
        Ok(Self(tiny_keccak::Keccak::v256()))
    }
}

impl Keccak_<48> for Keccak<48> {
    fn new() -> Result<Self, crate::Error> {
        Ok(Self(tiny_keccak::Keccak::v384()))
    }
}

impl Keccak_<64> for Keccak<64> {
    fn new() -> Result<Self, crate::Error> {
        Ok(Self(tiny_keccak::Keccak::v512()))
    }
}

impl<const S: usize> Keccak<S>
where
    Self: Keccak_<S>,
{
    pub const DIGEST_LEN: usize = S;

    pub fn new_gce(loc: &mut MaybeUninit<Self>) -> Result<(), crate::Error> {
        *loc = MaybeUninit::new(Self::new()?);

        Ok(())
    }

    pub fn new() -> Result<Self, crate::Error> {
        Keccak_::new()
    }
}

impl<const S: usize> super::Hasher<S> for Keccak<S>
where
    Self: Keccak_<S>,
{
    type Error = crate::Error;

    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.update(input);
        Ok(())
    }

    fn finalize_dirty_into(&mut self, out: &mut [u8; S]) -> Result<(), Self::Error> {
        let this = self.0.clone();

        this.finalize(out);

        Ok(())
    }

    fn finalize_into(self, out: &mut [u8; S]) -> Result<(), Self::Error> {
        self.0.finalize(out);

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        *self = Self::new()?;
        Ok(())
    }

    fn digest_into(input: &[u8], out: &mut [u8; S]) -> Result<(), Self::Error> {
        let mut hasher = Self::new()?;
        hasher.update(input)?;
        hasher.finalize_into(out)
    }
}

impl<const S: usize> super::HasherId for Keccak<S> {
    type Id = u8;

    fn id() -> Self::Id {
        6
    }
}
