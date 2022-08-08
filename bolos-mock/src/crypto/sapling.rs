pub fn fill_sapling_seed(_zip32_seed: &mut [u8])  {
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
    );}

