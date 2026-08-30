use crate::CoreError;
use crate::CoreResult;

pub const MAGIC: &[u8; 8] = b"CISTA\0\0\0";
pub const CURRENT_FORMAT_VERSION: u16 = 1;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdfId {
    Argon2id = 1,
}

impl TryFrom<u8> for KdfId {
    type Error = CoreError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(KdfId::Argon2id),
            _ => Err(CoreError::InvalidFormat),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AeadId {
    XChaCha20Poly1305 = 1,
}

impl TryFrom<u8> for AeadId {
    type Error = CoreError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AeadId::XChaCha20Poly1305),
            _ => Err(CoreError::InvalidFormat),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct VaultHeader {
    pub format_version: u16,
    pub kdf_id: KdfId,
    pub kdf_memory_kib: u32,
    pub kdf_iterations: u32,
    pub kdf_parallelism: u32,
    pub salt: [u8; SALT_LEN],
    pub aead_id: AeadId,
    pub nonce: [u8; NONCE_LEN],
}

impl VaultHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.format_version.to_le_bytes());
        bytes.push(self.kdf_id as u8);
        bytes.extend_from_slice(&self.kdf_memory_kib.to_le_bytes());
        bytes.extend_from_slice(&self.kdf_iterations.to_le_bytes());
        bytes.extend_from_slice(&self.kdf_parallelism.to_le_bytes());
        bytes.extend_from_slice(&self.salt);
        bytes.push(self.aead_id as u8);
        bytes.extend_from_slice(&self.nonce);
        bytes
    }
}

impl VaultHeader {
    pub fn from_bytes(bytes: &[u8]) -> CoreResult<(Self, usize)> {
        let mut offset = 0;

        let format_version = read_u16(bytes, &mut offset)?;
        let kdf_id = KdfId::try_from(read_u8(bytes, &mut offset)?)?;
        let kdf_memory_kib = read_u32(bytes, &mut offset)?;
        let kdf_iterations = read_u32(bytes, &mut offset)?;
        let kdf_parallelism = read_u32(bytes, &mut offset)?;
        let salt = read_array::<SALT_LEN>(bytes, &mut offset)?;
        let aead_id = AeadId::try_from(read_u8(bytes, &mut offset)?)?;
        let nonce = read_array::<NONCE_LEN>(bytes, &mut offset)?;

        let header = Self {
            format_version,
            kdf_id,
            kdf_memory_kib,
            kdf_iterations,
            kdf_parallelism,
            salt,
            aead_id,
            nonce,
        };

        Ok((header, offset))
    }
}

pub struct CistaFile {
    pub header: VaultHeader,
    pub ciphertext: Vec<u8>,
}

impl CistaFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&(self.ciphertext.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> CoreResult<Self> {
        let magic = bytes.get(0..8).ok_or(CoreError::InvalidFormat)?;
        if magic != MAGIC {
            return Err(CoreError::InvalidFormat);
        }

        let (header, consumed) = VaultHeader::from_bytes(&bytes[8..])?;
        let mut offset = 8 + consumed;

        let ciphertext_len = read_u32(bytes, &mut offset)? as usize;
        let ciphertext = bytes
            .get(offset..offset + ciphertext_len)
            .ok_or(CoreError::InvalidFormat)?
            .to_vec();

        Ok(Self { header, ciphertext })
    }
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> CoreResult<u8> {
    let byte = *bytes.get(*offset).ok_or(CoreError::InvalidFormat)?;
    *offset += 1;
    Ok(byte)
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> CoreResult<u16> {
    let slice = bytes
        .get(*offset..*offset + 2)
        .ok_or(CoreError::InvalidFormat)?;
    *offset += 2;
    let arr: [u8; 2] = slice.try_into().map_err(|_| CoreError::InvalidFormat)?;
    Ok(u16::from_le_bytes(arr))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> CoreResult<u32> {
    let slice = bytes
        .get(*offset..*offset + 4)
        .ok_or(CoreError::InvalidFormat)?;
    *offset += 4;
    let arr: [u8; 4] = slice.try_into().map_err(|_| CoreError::InvalidFormat)?;
    Ok(u32::from_le_bytes(arr))
}

fn read_array<const N: usize>(bytes: &[u8], offset: &mut usize) -> CoreResult<[u8; N]> {
    let slice = bytes
        .get(*offset..*offset + N)
        .ok_or(CoreError::InvalidFormat)?;
    *offset += N;
    slice.try_into().map_err(|_| CoreError::InvalidFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let header = VaultHeader {
            format_version: CURRENT_FORMAT_VERSION,
            kdf_id: KdfId::Argon2id,
            kdf_memory_kib: 19_456, // example
            kdf_iterations: 2,
            kdf_parallelism: 1,
            salt: [42u8; SALT_LEN],
            aead_id: AeadId::XChaCha20Poly1305,
            nonce: [7u8; NONCE_LEN],
        };

        let bytes = header.to_bytes();
        let (decoded, consumed) =
            VaultHeader::from_bytes(&bytes).expect("should decode a valid header");
        assert_eq!(header, decoded);
        assert_eq!(consumed, bytes.len());
    }

    #[test]
    fn header_from_truncated_bytes_fails() {
        let header = VaultHeader {
            format_version: CURRENT_FORMAT_VERSION,
            kdf_id: KdfId::Argon2id,
            kdf_memory_kib: 19_456,
            kdf_iterations: 2,
            kdf_parallelism: 1,
            salt: [42u8; SALT_LEN],
            aead_id: AeadId::XChaCha20Poly1305,
            nonce: [7u8; NONCE_LEN],
        };

        let bytes = header.to_bytes();
        let truncated = &bytes[..bytes.len() - 5]; // quit last-5 bytes

        let result = VaultHeader::from_bytes(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn kdf_id_rejects_unknown_value() {
        let result = KdfId::try_from(99u8);
        assert!(result.is_err());
    }
}
