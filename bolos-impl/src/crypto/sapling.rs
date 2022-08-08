use super::{bip32::BIP32Path, Curve, Mode, CHAIN_CODE_LEN};

pub const CTX_EXPAND_SEED_HASH_LEN: usize = 64;
pub const CTX_EXPAND_SEED_LEN: usize = 16;

pub const ZIP32_SEED_LEN: usize = 32;
pub const HDPATH_LEN_DEFAULT: usize = 5;

pub const MASK_HARDENED: u32 = 0x8000_0000;

pub fn fill_sapling_seed(zip32_seed: &mut [u8; ZIP32_SEED_LEN]) {
    zemu_sys::zemu_log_stack("fill_sapling_seed");

    // Get seed from Ed25519
    let path: [u32; HDPATH_LEN_DEFAULT] = [
        0x8000002c,
        0x80000085,
        MASK_HARDENED,
        MASK_HARDENED,
        MASK_HARDENED,
    ];

    let path = match BIP32Path::<{ HDPATH_LEN_DEFAULT }>::new(path) {
        Ok(path) => path,
        Err(_) => unsafe { core::hint::unreachable_unchecked() },
    };

    super::bindings::os_perso_derive_node_with_seed_key(
        Mode::BIP32,
        Curve::Ed25519,
        &path,
        zip32_seed,
        None,
    );
}
