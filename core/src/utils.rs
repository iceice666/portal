pub(crate) fn u8_array_to_u16(array: [u8; 2]) -> u16 {
    ((array[0] as u16) << 8) | array[1] as u16
}

pub(crate) fn u16_to_u8_array(value: u16) -> [u8; 2] {
    [(value >> 8) as u8, value as u8]
}

pub(crate) fn u64_to_u8_array(value: u64) -> [u8; 8] {
    [
        (value >> 56) as u8,
        (value >> 48) as u8,
        (value >> 40) as u8,
        (value >> 32) as u8,
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ]
}

pub(crate) fn u8_array_to_u64(array: [u8; 8]) -> u64 {
    ((array[0] as u64) << 56)
        | ((array[1] as u64) << 48)
        | ((array[2] as u64) << 40)
        | ((array[3] as u64) << 32)
        | ((array[4] as u64) << 24)
        | ((array[5] as u64) << 16)
        | ((array[6] as u64) << 8)
        | array[7] as u64
}
