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
use bolos_common::hash::HasherId;
use core::mem::MaybeUninit;

use crate::Error;

use super::{bip32::BIP32Path, Curve, Mode, CHAIN_CODE_LEN};

pub use enumflags2::BitFlags;

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum ECCInfo {
    /// Tells wheter the Y component as even (0) or odd (1)
    ParityOdd,
    /// Tells wheter the X component was greater than the curve order
    XGTn,
}

#[derive(Clone, Copy)]
pub struct PublicKey {
    curve: Curve,
    len: usize,
    data: [u8; 65],
}

impl PublicKey {
    pub fn compress(&mut self) -> Result<(), Error> {
        match self.curve {
            Curve::Secp256K1 => {
                let point = k256::EncodedPoint::from_bytes(&self.data[..self.len]).unwrap();
                let compressed = point.compress();

                self.len = 33;
                self.data[..33].copy_from_slice(compressed.as_ref());
                Ok(())
            }
            Curve::Secp256R1 => {
                let point = p256::EncodedPoint::from_bytes(&self.data[..self.len]).unwrap();
                let compressed = point.compress();

                self.len = 33;
                self.data[..33].copy_from_slice(compressed.as_ref());
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn curve(&self) -> Curve {
        self.curve
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
    curve: Curve,
    bytes: [u8; 32],
}

impl<const B: usize> SecretKey<B> {
    fn rng8(path: BIP32Path<B>) -> rand_chacha8::ChaCha8Rng {
        use rand_chacha8::rand_core::SeedableRng;

        let mut seed = [0; 32 / 4];
        for (i, c) in path.components().iter().enumerate().take(32 / 4) {
            seed[i] = *c;
        }

        let seed: [u8; 32] = unsafe { std::mem::transmute(seed) };

        rand_chacha8::ChaCha8Rng::from_seed(seed)
    }

    fn rng7(path: BIP32Path<B>) -> rand_chacha7::ChaCha8Rng {
        use rand_chacha7::rand_core::SeedableRng;

        let mut seed = [0; 32 / 4];
        for (i, c) in path.components().iter().enumerate().take(32 / 4) {
            seed[i] = *c;
        }

        let seed: [u8; 32] = unsafe { std::mem::transmute(seed) };

        rand_chacha7::ChaCha8Rng::from_seed(seed)
    }

    pub fn new(_: Mode, curve: Curve, path: BIP32Path<B>) -> Self {
        let bytes = match curve {
            Curve::Secp256K1 => {
                let secret = k256::ecdsa::SigningKey::random(&mut Self::rng8(path));

                *secret.to_bytes().as_ref()
            }
            Curve::Secp256R1 => {
                let secret = p256::ecdsa::SigningKey::random(&mut Self::rng8(path));

                *secret.to_bytes().as_ref()
            }
            Curve::Ed25519 => {
                let mut bytes = [0u8; 32];
                let mut rng = Self::rng8(path);
                use rand_chacha8::rand_core::RngCore;
                rng.fill_bytes(&mut bytes);
                bytes
            }
            Curve::Stark256 => {
                panic!("invalid curve passed to ecfp256 new")
            }
        };

        Self { curve, bytes }
    }

    pub const fn curve(&self) -> Curve {
        self.curve
    }

    pub fn public(&self) -> Result<PublicKey, Error> {
        let (data, len) = match self.curve {
            Curve::Secp256K1 => {
                let secret = k256::ecdsa::SigningKey::from_slice(&self.bytes[..]).unwrap();

                let public = secret.verifying_key();
                //when we encode we don't compress the point right away
                let uncompressed_point = public.to_encoded_point(false);
                let uncompressed_point = uncompressed_point.as_bytes();

                let mut bytes = [0; 65];
                bytes[..uncompressed_point.len()].copy_from_slice(uncompressed_point);

                (bytes, uncompressed_point.len())
            }
            Curve::Secp256R1 => {
                let secret = p256::ecdsa::SigningKey::from_slice(&self.bytes[..]).unwrap();

                let public = secret.verifying_key();
                //when we encode we don't compress the point right away
                let uncompressed_point = public.to_encoded_point(false);
                let uncompressed_point = uncompressed_point.as_ref();

                let mut bytes = [0; 65];
                bytes[..uncompressed_point.len()].copy_from_slice(uncompressed_point);

                (bytes, uncompressed_point.len())
            }
            Curve::Ed25519 => {
                let secret = ed25519_dalek::SigningKey::from_bytes(&self.bytes);
                let public = secret.verifying_key();
                let mut bytes = [0; 65];
                bytes[..32].copy_from_slice(public.as_bytes());
                (bytes, 32)
            }
            _ => unreachable!(),
        };

        Ok(PublicKey {
            curve: self.curve,
            data,
            len,
        })
    }

    pub fn public_into(
        &self,
        _: Option<&mut [u8; CHAIN_CODE_LEN]>,
        out: &mut MaybeUninit<PublicKey>,
    ) -> Result<(), Error> {
        let pk = self.public()?;

        *out = MaybeUninit::new(pk);

        Ok(())
    }

    pub fn sign<H>(&self, data: &[u8], out: &mut [u8]) -> Result<(BitFlags<ECCInfo>, usize), Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        match self.curve {
            Curve::Secp256K1 => {
                use k256::ecdsa::{signature::Signer, Signature};

                let secret = k256::ecdsa::SigningKey::from_bytes((&self.bytes[..]).into()).unwrap();

                let sig: Signature = secret.sign(data);
                let sig = sig.to_der();
                let sig = sig.as_ref();

                out[..sig.len()].copy_from_slice(sig);
                Ok((Default::default(), sig.len()))
            }
            Curve::Secp256R1 => {
                use p256::ecdsa::{signature::Signer, Signature};

                let secret = p256::ecdsa::SigningKey::from_bytes((&self.bytes[..]).into()).unwrap();
                let sig: Signature = secret.sign(data);
                let sig = sig.to_der();
                let sig = sig.as_ref();

                out[..sig.len()].copy_from_slice(sig);
                Ok((Default::default(), sig.len()))
            }
            Curve::Ed25519 => {
                use ed25519_dalek::Signer;
                let secret = ed25519_dalek::SigningKey::from_bytes(&self.bytes);
                let sig = secret.sign(data);
                out[..64].copy_from_slice(&sig.to_bytes()[..]);
                Ok((Default::default(), 64))
            }
            _ => unreachable!(),
        }
    }
}
