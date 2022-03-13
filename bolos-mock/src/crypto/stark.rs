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
use bolos_common::hash::HasherId;
use core::mem::MaybeUninit;

use crate::Error;

use super::{bip32::BIP32Path, Curve, Mode};

#[derive(Clone, Copy)]
pub struct PublicKey {
    len: usize,
    data: [u8; 65],
}

impl PublicKey {
    pub fn curve(&self) -> Curve {
        Curve::Stark256
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.data[..self.len]
    }
}

pub struct SecretKey<const B: usize> {
    bytes: [u8; 32],
}

impl<const B: usize> SecretKey<B> {
    pub fn new(_: BIP32Path<B>) -> Self {
        let bytes = [0; 32];

        todo!("starkware new secret");
        Self { bytes }
    }

    pub const fn curve(&self) -> Curve {
        Curve::Stark256
    }

    pub fn public(&self) -> Result<PublicKey, Error> {
        let (data, len) = ([0; 65], 0);

        todo!("starkware to public");

        Ok(PublicKey { data, len })
    }

    pub fn public_into(&self, out: &mut MaybeUninit<PublicKey>) -> Result<(), Error> {
        let pk = self.public()?;

        *out = MaybeUninit::new(pk);

        Ok(())
    }

    pub fn sign(&self, data: &[u8], out: &mut [u8]) -> Result<(bool, usize), Error> {
        todo!("starkware sign")
    }
}
