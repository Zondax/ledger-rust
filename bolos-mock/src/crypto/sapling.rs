const MASK_HARDENED: u32 = 0x8000_0000;

const ZIP32_PATH: [u32; 5] = [
    0x8000002c,
    0x80000085,
    MASK_HARDENED,
    MASK_HARDENED,
    MASK_HARDENED,
];

pub const ZIP32_SEED_LEN: usize = 32;

pub fn fill_sapling_seed(zip32_seed: &mut [u8; ZIP32_SEED_LEN]) {
    let partial_seed: [u8; 5 * 4] = bytemuck::cast(ZIP32_PATH);

    zip32_seed[..5 * 4].copy_from_slice(&partial_seed);
    zip32_seed[5 * 4..].copy_from_slice(&[0; ZIP32_SEED_LEN - (5 * 4)]);
}
