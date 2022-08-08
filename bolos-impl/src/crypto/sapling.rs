use super::{bip32::BIP32Path, Curve, Mode, CHAIN_CODE_LEN};
use jubjub;

pub const CTX_EXPAND_SEED_HASH_LEN : size_t = 64;
pub const CTX_EXPAND_SEED_LEN : size_t = 16;

pub fn fill_sapling_seed(zip32_seed: &mut [u8])  {
    zemu_log_stack("fill_sapling_seed");
// Get seed from Ed25519
    let path: [u32;HDPATH_LEN_DEFAULT] = [0x8000002c,
        0x80000085,
        MASK_HARDENED,
        MASK_HARDENED,
        MASK_HARDENED];

    os_perso_derive_node_with_seed_key(
        Mode::BIP32,
        Curve::Ed25519,
        path, HDPATH_LEN_DEFAULT,
        zip32_seed,
        None,
        None,
        0
    );
}


mod bindings {
    use super::{bip32::BIP32Path, Curve, Mode};
    use crate::errors::{catch, Error};

    pub fn os_perso_derive_node_with_seed_key<const B: usize>(
        mode: Mode,
        curve: Curve,
        path: &BIP32Path<B>,
        out: &mut [u8],
        chain: Option<&mut [u8; 32]>,
    ) -> Result<(), Error> {
        zemu_sys::zemu_log_stack("os_perso_derive_node_with_seed_key\x00");
        let curve: u8 = curve.into();
        let mode: u8 = mode.into();

        let out_p = out.as_mut_ptr();
        let (components, path_len) = {
            let components = path.components();
            (components.as_ptr(), components.len() as u32)
        };

        let chain = if let Some(chain) = chain {
            chain.as_mut_ptr()
        } else {
            std::ptr::null_mut()
        };

        cfg_if! {
            if #[cfg(bolos_sdk)] {
                let might_throw = || unsafe {
                    crate::raw::os_perso_derive_node_with_seed_key(
                        mode as _,
                        curve as _,
                        components as *const _,
                        path_len as _,
                        out_p as *mut _,
                        chain as *mut _,
                        std::ptr::null_mut(),
                        0
                    )
                };

                catch(might_throw)?;
            } else {
                unimplemented!("os derive called in non-bolos")
            }
        }

        Ok(())
    }
}