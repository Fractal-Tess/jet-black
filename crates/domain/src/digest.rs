use sha2::{Digest, Sha256};

pub(crate) fn update_len_prefixed(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}
