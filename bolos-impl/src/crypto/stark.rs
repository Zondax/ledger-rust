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
use zeroize::{Zeroize, Zeroizing};

use super::{bip32::BIP32Path, Curve, Mode};
use crate::{
    errors::Error,
    raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
};

use core::{mem::MaybeUninit, ptr::addr_of_mut};

#[derive(Clone, Copy)]
pub struct PublicKey(cx_ecfp_public_key_t);

impl PublicKey {
    pub fn curve(&self) -> Curve {
        Curve::Stark256
    }

    pub fn len(&self) -> usize {
        self.0.W_len as usize
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0.W[..self.0.W_len as usize]
    }
}

pub struct SecretKey<const B: usize> {
    path: BIP32Path<B>,
}

const SIGNATURE_MAX_LEN: usize = 72;

impl<const B: usize> SecretKey<B> {
    pub const SIGNATURE_MAX_LEN: usize = SIGNATURE_MAX_LEN;

    pub const fn new(path: BIP32Path<B>) -> Self {
        Self { path }
    }

    #[inline(always)]
    pub const fn curve(&self) -> Curve {
        Curve::Stark256
    }

    #[inline(never)]
    fn generate(&self) -> Result<Zeroizing<cx_ecfp_private_key_t>, Error> {
        let mut out = MaybeUninit::uninit();

        self.generate_into(&mut out)?;

        Ok(Zeroizing::new(unsafe { out.assume_init() }))
    }

    fn generate_into(&self, out: &mut MaybeUninit<cx_ecfp_private_key_t>) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("SecretKey::generate_into\x00");

        // Prepare secret key data with the ledger's key
        let mut sk_data = [0; 32];

        bindings::stark_derive_node(&self.path, &mut sk_data)?;

        // Use the secret key data to prepare a secret key
        let sk_r =
            super::ecfp256::cx_ecfp_init_private_key_into(self.curve(), Some(&sk_data[..]), out);
        // let's zeroize the sk_data right away before we return
        sk_data.zeroize();

        sk_r
    }

    #[inline(never)]
    pub fn public(&self) -> Result<PublicKey, Error> {
        let mut out = MaybeUninit::uninit();

        self.public_into(&mut out)?;

        //this is safe as the call above initialized it
        Ok(unsafe { out.assume_init() })
    }

    #[inline(never)]
    pub fn public_into(&self, out: &mut MaybeUninit<PublicKey>) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("SecretKey::public_into\x00");

        let pk = {
            let out = out.as_mut_ptr();

            unsafe {
                //retrive the inner section and cast it as MaybeUninit
                match addr_of_mut!((*out).0).cast::<MaybeUninit<_>>().as_mut() {
                    Some(ptr) => ptr,
                    None => core::hint::unreachable_unchecked(), //pointer is guaranteed valid
                }
            }
        };

        let mut sk = MaybeUninit::uninit();
        //get keypair with the generated secret key
        // discard secret key as it's not necessary anymore
        stark_generate_pair_into(self, &mut sk, pk)?;
        //SAFE: sk is initialized
        unsafe { sk.assume_init() }.zeroize();

        Ok(())
    }

    #[inline(never)]
    /// Signs a given message with the stark curve
    ///
    /// Returns (ECC_PARITY_ODD, sig_size)
    pub fn sign(&self, data: &[u8], out: &mut [u8]) -> Result<(bool, usize), Error> {
        stark_sign::<B>(self, data, out)
    }
}

mod bindings {
    #![allow(unused_imports)]

    use super::{BIP32Path, Curve, Error, Mode, SecretKey};
    use crate::{
        errors::catch,
        hash::{Hasher, HasherId, Sha256},
        math,
        raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
        PIC,
    };
    use core::mem::MaybeUninit;
    use zeroize::Zeroizing;

    // C_cx_secp256k1_n - (C_cx_secp256k1_n % C_cx_Stark256_n)
    const STARK_DERIVE_BIAS: &[u8] = &[
        0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x0e, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xf7, 0x38, 0xa1, 0x3b, 0x4b, 0x92, 0x0e, 0x94, 0x11, 0xae, 0x6d, 0xa5, 0xf4, 0x0b, 0x03,
        0x58, 0xb1,
    ];

    // n: 0x0800000000000010ffffffffffffffffb781126dcae7b2321e66a241adc64d2f
    const C_CX_STARK256_N: &[u8] = &[
        0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xb7, 0x81, 0x12, 0x6d, 0xca, 0xe7, 0xb2, 0x32, 0x1e, 0x66, 0xa2, 0x41, 0xad, 0xc6,
        0x4d, 0x2f,
    ];

    pub fn stark_derive_node<const B: usize>(
        path: &BIP32Path<B>,
        out: &mut [u8; 32],
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("stark_derive_node\x00");

        //wrap in Zeroize so we guarantee zeroization on drop
        let mut tmp_secret = Zeroizing::new([0; 33]);
        let mut index = 0;

        crate::crypto::bindings::os_perso_derive_node_with_seed_key(
            Mode::BIP32,
            Curve::Secp256K1,
            &path,
            &mut *tmp_secret,
        )?;

        //PIC STARK_DERIVE_BIAS
        let stark_derive_bias = {
            let data = STARK_DERIVE_BIAS;
            let data_len = data.len();

            let to_pic = data.as_ptr() as usize;
            let picced = unsafe { PIC::manual(to_pic) } as *const ();

            //cast to same type as `to_pic`
            let ptr = picced.cast();
            unsafe { ::core::slice::from_raw_parts(ptr, data_len) }
        };

        //PIC C_CX_STARK256_N
        let c_cx_stark256_n = {
            let data = C_CX_STARK256_N;
            let data_len = data.len();

            let to_pic = data.as_ptr() as usize;
            let picced = unsafe { PIC::manual(to_pic) } as *const ();

            //cast to same type as `to_pic`
            let ptr = picced.cast();
            unsafe { ::core::slice::from_raw_parts(ptr, data_len) }
        };

        //TODO: document the process behind this
        // taken from original and converted to rust
        // https://github.com/LedgerHQ/app-ethereum/blob/a53a2428cc024855ef2012ca357e054f838ab962/src/stark_crypto.c#L18
        loop {
            tmp_secret[32] = index;
            Sha256::digest_into(&*tmp_secret, out)?;

            if math::cmp(&out[..], stark_derive_bias)?.is_lt() {
                math::modm(out, c_cx_stark256_n)?;
                break;
            }

            index += 1;
        }

        Ok(())
    }

    pub fn stark_generate_pair_into<const B: usize>(
        sk: &SecretKey<B>,
        out_sk: &mut MaybeUninit<cx_ecfp_private_key_t>,
        out_pk: &mut MaybeUninit<cx_ecfp_public_key_t>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("stark_generate_pair\x00");
        let curve: u8 = Curve::Stark256.into();

        sk.generate_into(out_sk)?;
        let keep = true;

        let raw_sk = out_sk.as_mut_ptr();
        let pk = out_pk.as_mut_ptr();

        cfg_if! {
            if #[cfg(any(nanox, nanos))] {
                match unsafe { crate::raw::cx_ecfp_generate_pair_no_throw(
                    curve as _,
                    pk,
                    raw_sk,
                    keep,
                )} {
                    0 => (),
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("generate_stark_keypair called in non-bolos");
            }
        }

        Ok(())
    }

    pub fn stark_sign<const B: usize>(
        sk: &SecretKey<B>,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<(bool, usize), Error> {
        use crate::raw::CX_RND_RFC6979;

        let id: u8 = Sha256::id().into();

        let crv = Curve::Stark256;

        let mut raw_sk = sk.generate()?;
        let raw_sk: *mut cx_ecfp_private_key_t = &mut *raw_sk;
        let raw_sk = raw_sk as *const _;

        let (data, data_len) = (data.as_ptr(), data.len() as u32);
        let sig = sig_out.as_mut_ptr();

        let mut sig_len = match crv.domain_length() {
            Some(n) => 6 + 2 * (n + 1),
            None => sig_out.len(),
        } as u32;

        let mut info = 0;

        cfg_if! {
            if #[cfg(any(nanox, nanos))] {
                match unsafe { crate::raw::cx_ecdsa_sign_no_throw(
                    raw_sk,
                    CX_RND_RFC6979,
                    id as _,
                    data,
                    data_len as _,
                    sig,
                    &mut sig_len as *mut _,
                    &mut info as *mut u32 as *mut _,
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("cx_ecdsa_sign called in not bolos")
            }
        }

        Ok((info == crate::raw::CX_ECCINFO_PARITY_ODD, sig_len as usize))
    }
}
use bindings::*;
