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

use super::{bip32::BIP32Path, Curve, Mode, CHAIN_CODE_LEN};
use crate::{
    errors::Error,
    hash::HasherId,
    raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
};

use core::{mem::MaybeUninit, ptr::addr_of_mut};

#[derive(Clone, Copy)]
pub struct PublicKey(cx_ecfp_public_key_t);

// Taken from lcx_ecfp.h
/**
The value shall be the public point encoded as:
 * - **04 || x || y** for Weiertrass curves
 * - **04 || x || y**  or **02 || y** (plus sign) for Twisted Edward curves
 * - **04 || x || y**  or **02 || x** for Montgomery curves
*/

impl PublicKey {
    /// Retrieve the `x` component of the public key
    #[allow(dead_code)]
    fn x(&self) -> Result<&[u8], Error> {
        let curve = self.curve();

        //CX_INVALID_CURVE, should only be for stark...
        let domain = curve.domain_length().ok_or(Error::from(0xFFFFFFA3u32))?;

        match self.0.W[0] {
            04 => Ok(&self.0.W[1..][..domain]),
            //CX_INVALID_PARAMTER, compressed so it's missing
            02 if curve.is_twisted_edward() => Err(Error::from(0xFFFFFF88u32)),
            02 if curve.is_montgomery() => Ok(&self.0.W[1..][..domain]),
            _ => Err(Error::from(0xFFFFFF87u32)), //CX_INVALID_PARAMETER_VALUE
        }
    }

    /// Retrieve the `y` component of the public key
    #[allow(dead_code)]
    fn y(&self) -> Result<&[u8], Error> {
        let curve = self.curve();

        //CX_INVALID_CURVE, should only be for stark...
        let domain = curve.domain_length().ok_or(Error::from(0xFFFFFFA3u32))?;

        match self.0.W[0] {
            04 => Ok(&self.0.W[1 + domain..][..domain]),
            02 if curve.is_twisted_edward() => Ok(&self.0.W[1 + domain..][..domain]),
            //CX_INVALID_PARAMTER, compressed so it's missing
            02 if curve.is_montgomery() => Err(Error::from(0xFFFFFF88u32)),
            _ => Err(Error::from(0xFFFFFF87u32)), //CX_INVALID_PARAMETER_VALUE
        }
    }

    pub fn compress(&mut self) -> Result<(), Error> {
        match self.curve() {
            c if c.is_weirstrass() => {
                //TODO: change to call cx_ecpoint_compress

                //tag:
                // 0x00 identity
                // 0x02 compressed, even Y
                // 0x03 compressed, odd Y
                // 0x04 uncompressed

                //assume it's compressed
                // should never be identity or compact anyways
                if self.0.W[0] != 4 {
                    return Ok(());
                }

                //-1 on the len since otherwise it would access the 65th byte
                let is_odd = self.0.W[(self.0.W_len as usize) - 1] & 0x01 > 0;
                self.0.W[0] = 0x02 + is_odd as u8;
                // cut the length to just 1 + 32 bytes (1 + x_len)
                self.0.W_len = 33;
                Ok(())
            }
            Curve::Ed25519 => {
                let comp_len = cx_edward_compress_point(Curve::Ed25519, &mut self.0.W[..])?;
                self.0.W_len = comp_len as _;

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn curve(&self) -> Curve {
        use core::convert::TryFrom;

        match Curve::try_from(self.0.curve as u8) {
            Ok(c) => c,
            //SAFE: we checked the curve already
            // nobody else can write this legally
            Err(_) => unsafe { core::hint::unreachable_unchecked() },
        }
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
    mode: Mode,
    curve: Curve,
    path: BIP32Path<B>,
}

impl<const B: usize> SecretKey<B> {
    /// Create a new secret key handle.
    ///
    /// No secret bytes are actually present within this struct, but rather,
    /// the secret key is actually computed, based on the data stored here,
    /// before each operation requiring said secret bytes.
    pub const fn new(mode: Mode, curve: Curve, path: BIP32Path<B>) -> Self {
        Self { mode, curve, path }
    }

    pub const fn curve(&self) -> Curve {
        self.curve
    }

    #[inline(never)]
    fn generate(
        &self,
        chaincode: Option<&mut [u8; CHAIN_CODE_LEN]>,
    ) -> Result<Zeroizing<cx_ecfp_private_key_t>, Error> {
        let mut out = MaybeUninit::uninit();

        self.generate_into(chaincode, &mut out)?;

        Ok(Zeroizing::new(unsafe { out.assume_init() }))
    }

    fn generate_into(
        &self,
        chaincode: Option<&mut [u8; CHAIN_CODE_LEN]>,
        out: &mut MaybeUninit<cx_ecfp_private_key_t>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("SecretKey::generate_into\x00");
        // Prepare secret key data with the ledger's key
        let mut sk_data = [0; 64];

        super::bindings::os_perso_derive_node_with_seed_key(
            self.mode,
            self.curve,
            &self.path,
            &mut sk_data,
            chaincode,
        )?;

        // Use the secret key data to prepare a secret key
        let sk_r = cx_ecfp_init_private_key_into(self.curve, Some(&sk_data[..]), out);
        // let's zeroize the sk_data right away before we return
        sk_data.zeroize();

        sk_r
    }

    #[inline(never)]
    pub fn public(&self) -> Result<PublicKey, Error> {
        let mut out = MaybeUninit::uninit();

        self.public_into(None, &mut out)?;

        //this is safe as the call above initialized it
        Ok(unsafe { out.assume_init() })
    }

    #[inline(never)]
    pub fn public_into(
        &self,
        chaincode: Option<&mut [u8; CHAIN_CODE_LEN]>,
        out: &mut MaybeUninit<PublicKey>,
    ) -> Result<(), Error> {
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
        cx_ecfp_generate_pair_into(Some(self), self.curve, chaincode, &mut sk, pk)?;
        //SAFE: sk is initialized
        unsafe { sk.assume_init() }.zeroize();

        Ok(())
    }

    #[inline(never)]
    pub fn sign<H>(&self, data: &[u8], out: &mut [u8]) -> Result<usize, Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        let crv = self.curve;
        if crv.is_weirstrass() {
            let (parity, size) = bindings::cx_ecdsa_sign::<H, B>(self, data, out)?;
            if parity {
                out[0] |= 0x01;
            }

            Ok(size)
        } else if crv.is_twisted_edward() {
            bindings::cx_eddsa_sign(self, data, out)
        } else if crv.is_montgomery() {
            todo!("montgomery sign")
        } else {
            todo!("unknown signature type")
        }
    }
}

mod bindings {
    #![allow(unused_imports)]

    use super::{Curve, Error, HasherId, SecretKey, CHAIN_CODE_LEN};
    use crate::{
        errors::catch,
        raw::{cx_ecfp_private_key_t, cx_ecfp_public_key_t},
    };
    use core::mem::MaybeUninit;
    use zeroize::{Zeroize, Zeroizing};

    pub fn cx_edward_compress_point(curve: Curve, p: &mut [u8]) -> Result<usize, Error> {
        let curve: u8 = curve.into();

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                match unsafe { crate::raw::cx_edwards_compress_point_no_throw(
                    curve as _,
                    p.as_mut_ptr() as *mut _,
                    p.len() as u32 as _
                )} {
                    0 => Ok(33),
                    err => Err(err.into())
                }
            } else {
                unimplemented!("edwards_compress_point called in non-bolos");
            }
        }
    }

    #[allow(dead_code)]
    pub fn cx_ecfp_init_private_key(
        curve: Curve,
        sk_data: Option<&[u8]>,
    ) -> Result<cx_ecfp_private_key_t, Error> {
        let mut out = MaybeUninit::uninit();
        cx_ecfp_init_private_key_into(curve, sk_data, &mut out)?;

        //this is safe because the data is now initialized
        Ok(unsafe { out.assume_init() })
    }

    pub fn cx_ecfp_init_private_key_into(
        curve: Curve,
        sk_data: Option<&[u8]>,
        out: &mut MaybeUninit<cx_ecfp_private_key_t>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("cx_ecfp_init_private_key_into\x00");
        let curve: u8 = curve.into();

        let sk_data: *const u8 = match sk_data {
            None => std::ptr::null(),
            Some(data) => data.as_ptr(),
        };

        let out = out.as_mut_ptr();

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                match unsafe { crate::raw::cx_ecfp_init_private_key_no_throw(
                    curve as _,
                    sk_data as *const _,
                    32 as _,
                    out,
                )} {
                    0 => {},
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("init ecfp_private_key called in non-bolos");
            }
        }

        Ok(())
    }

    pub fn cx_ecfp_generate_pair_into<const B: usize>(
        sk: Option<&SecretKey<B>>,
        curve: Curve,
        chaincode: Option<&mut [u8; CHAIN_CODE_LEN]>,
        out_sk: &mut MaybeUninit<cx_ecfp_private_key_t>,
        out_pk: &mut MaybeUninit<cx_ecfp_public_key_t>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("cx_ecfp_generate_pair\x00");
        let curve: u8 = curve.into();

        let keep = match sk {
            Some(sk) => {
                sk.generate_into(chaincode, out_sk)?;
                true
            }
            None => {
                //no need to write in `raw_sk`,
                // since the function below will override everything
                false
            }
        };

        let raw_sk = out_sk.as_mut_ptr();
        let pk = out_pk.as_mut_ptr();

        cfg_if! {
            if #[cfg(bolos_sdk)] {
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
                unimplemented!("generate_ecfp_keypair called in non-bolos");
            }
        }

        Ok(())
    }

    //first item says if Y is odd when computing k.G
    // second item in the tuple is the number of bytes written to `sig_out`
    pub fn cx_ecdsa_sign<H, const B: usize>(
        sk: &SecretKey<B>,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<(bool, usize), Error>
    where
        H: HasherId,
        H::Id: Into<u8>,
    {
        use crate::raw::CX_RND_RFC6979;

        let id: u8 = H::id().into();

        let crv = sk.curve();

        let mut raw_sk = sk.generate(None)?;
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
            if #[cfg(bolos_sdk)] {
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

    pub fn cx_eddsa_sign<const B: usize>(
        sk: &SecretKey<B>,
        data: &[u8],
        sig_out: &mut [u8],
    ) -> Result<usize, Error> {
        let id: u8 = crate::hash::Sha512::id().into();

        let crv = sk.curve();

        let mut raw_sk = sk.generate(None)?;
        let raw_sk: *mut cx_ecfp_private_key_t = &mut *raw_sk;
        let raw_sk = raw_sk as *const _;

        let (data, data_len) = (data.as_ptr(), data.len() as u32);
        let sig = sig_out.as_mut_ptr();

        let mut sig_len = match crv.domain_length() {
            Some(n) => 6 + 2 * (n + 1),
            None => sig_out.len(),
        } as u32;

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                match unsafe { crate::raw::cx_eddsa_sign_no_throw(
                    raw_sk,
                    id as _,
                    data,
                    data_len as _,
                    sig,
                    sig_len as _,
                )} {
                    0 => {
                        let crv: u8 = crv.into();
                        match unsafe { crate::raw::cx_ecdomain_parameters_length(
                            crv as _,
                            &mut sig_len as *mut _
                        )} {
                            0 => {sig_len *= 2;},
                            err => return Err(err.into()),
                        }
                    },
                    err => return Err(err.into()),
                }
            } else {
                unimplemented!("cx_eddsa_sign called in not bolos")
            }
        }

        Ok(sig_len as usize)
    }
}
pub(super) use bindings::*;
