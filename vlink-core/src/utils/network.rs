pub fn bit_length(number: u32) -> u8 {
    32 - number.leading_zeros() as u8
}

pub fn bite_mask(mask: u8) -> u32 {
    debug_assert!(mask <= 32);
    match mask {
        0 => 0,
        n => !0 << (32 - n),
    }
}

pub fn bite_mask_u128(mask: u8) -> u128 {
    debug_assert!(mask <= 128);
    match mask {
        0 => 0,
        n => !0 << (128 - n),
    }
}