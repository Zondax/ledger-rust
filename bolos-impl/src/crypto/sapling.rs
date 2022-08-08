use super::{bip32::BIP32Path, Curve, Mode, CHAIN_CODE_LEN};
use jubjub;

pub const CTX_EXPAND_SEED_HASH_LEN: size_t = 64;
pub const CTX_EXPAND_SEED_LEN: size_t = 16;

pub const ZIP32_SEED_LEN: usize = 32;

pub fn fill_sapling_seed(zip32_seed: &mut [u8; ZIP32_SEED_LEN]) {
    zemu_log_stack("fill_sapling_seed");
    // Get seed from Ed25519
    let path: [u32; HDPATH_LEN_DEFAULT] = [
        0x8000002c,
        0x80000085,
        MASK_HARDENED,
        MASK_HARDENED,
        MASK_HARDENED,
    ];

    super::bindings::os_perso_derive_node_with_seed_key(
        Mode::BIP32,
        Curve::Ed25519,
        path,
        HDPATH_LEN_DEFAULT,
        zip32_seed,
        None,
        None,
        0,
    );
}
