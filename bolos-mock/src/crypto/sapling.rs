use blake2b_simd::Params as Blake2bParams;


pub fn fill_sapling_seed(_zip32_seed: &mut [u8])  {
    todo!("fill sapling seed");
}

pub fn blake2b_expand_seed(a: &[u8], b: &[u8], perso: &[u8;16] ) -> [u8; 64] {

    let h = Blake2bParams::new()
        .hash_length(64)
        .personal(perso as &[u8])
        .to_state()
        .update(a)
        .update(b)
        .finalize();

    let result: [u8; 64] = *h.as_array();
    result
}
