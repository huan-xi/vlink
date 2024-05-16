use std::ptr;
use libc::c_int;
use openssl_sys::EVP_CIPHER_CTX;
use crate::noise::Error;

pub struct AeadChacha20Cipher {
    key: Vec<u8>,
    pub(crate) en_ctx: *mut EVP_CIPHER_CTX,
    pub(crate) de_ctx: *mut EVP_CIPHER_CTX,
    // pub(crate) finger: Option<Finger>,
}

impl AeadChacha20Cipher {
    pub fn new(key: [u8; 32]) -> Self {
        unsafe {
            let cipher = openssl_sys::EVP_chacha20_poly1305();
            let en_ctx = openssl_sys::EVP_CIPHER_CTX_new();
            openssl_sys::EVP_EncryptInit_ex(
                en_ctx,
                cipher,
                ptr::null_mut(),
                key.as_ptr(),
                ptr::null(),
            );
            let de_ctx = openssl_sys::EVP_CIPHER_CTX_new();
            openssl_sys::EVP_DecryptInit_ex(
                de_ctx,
                cipher,
                ptr::null_mut(),
                key.as_ptr(),
                ptr::null(),
            );
            Self {
                key: key.to_vec(),
                en_ctx,
                de_ctx,
                // finger,
            }
        }
    }
    pub fn encrypt(&self, counter: u64, data: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
        let mut out = [0u8; 1024 * 5];
        let mut out_len = 0;
        let ctx = self.en_ctx;
        unsafe {
            let out_ptr = out.as_mut_ptr();

            let nonce = {
                let mut nonce = [0u8; 12];
                nonce[4..].copy_from_slice(&counter.to_le_bytes());
                nonce
            };
            // openssl_sys::EVP_EncryptUpdate(ctx, out_ptr, &mut out_len, data.as_ptr(), data_len);



            let data_len = data.len() as c_int;
            openssl_sys::EVP_EncryptUpdate(ctx, out_ptr, &mut out_len, data.as_ptr(), data_len);
            let mut last_len = 0;
            openssl_sys::EVP_EncryptFinal_ex(ctx, out_ptr.offset(out_len as isize), &mut last_len);
            out_len += last_len;
        }
        // Ok(out[..out_len].to_vec())
        todo!();
    }
}