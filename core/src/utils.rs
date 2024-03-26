pub(crate) fn u8_array_to_u16(array: [u8; 2]) -> u16 {
    ((array[0] as u16) << 8) | array[1] as u16
}

pub(crate) fn u16_to_u8_array(value: u16) -> [u8; 2] {
    [(value >> 8) as u8, value as u8]
}
