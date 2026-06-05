/// Calculates the standard CRC-16-CCITT checksum (polynomial 0x1021, initial value 0xFFFF)
/// over the provided data.
pub fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_standard_vectors() {
        // Standard ASCII "123456789" vector should yield 0x29B1
        assert_eq!(calculate_crc16(b"123456789"), 0x29B1);
    }
}
