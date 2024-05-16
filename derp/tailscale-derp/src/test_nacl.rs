// use nacl::public_box::{KEY_LENGTH, NONCE_LENGTH, pack, POLY_LENGTH};
// use nacl::sha512::hash_sha512;
// use nacl::sign::{extract_pkey, generate_keypair, Keypair};

use crypto_box::aead::{Aead, AeadCore, Key, KeyInit, OsRng};
use crypto_box::{KEY_SIZE, Nonce, SalsaBox, SecretKey};
#[cfg(test)]
#[test]
pub fn test_key() {
    let key = SecretKey::generate(&mut OsRng);
    println!("{:?}", key.public_key());

}
#[cfg(test)]
#[test]
pub fn test_nacl() {
    //[1u8; KEY_SIZE]
    let msg = [3u8; 64];



    let alice_secret_key = SecretKey::from([1u8; KEY_SIZE]);
    let alice_public_key_bytes = alice_secret_key.public_key().as_bytes().clone();

    let bob_secret_key = SecretKey::from([2u8; KEY_SIZE]);
    let bob_public_key = bob_secret_key.public_key();


    let alice_box = SalsaBox::new(&bob_public_key, &alice_secret_key);
    let nonce = [4u8; 24];
    let no = Nonce::from(nonce);
    // alice_box.decrypt();
    let ciphertext = alice_box.encrypt(&no, &msg[..]).unwrap();
    let result = [120, 234, 48, 177, 157, 35, 65, 235, 189, 186, 84, 24, 15, 130, 30, 236, 38, 92, 248, 99, 18, 84, 155, 234, 138, 55, 101, 42, 139, 185, 79, 7, 183, 138, 115, 237, 23, 8, 8, 94, 109, 221, 14, 148, 59, 189, 235, 135, 85, 7, 154, 55, 235, 49, 216, 97, 99, 206, 36, 17, 100, 164, 118, 41, 192, 83, 159, 51, 11, 73, 20, 205, 19, 91, 56, 85, 188, 42, 45, 252];
    assert_eq!(ciphertext, result);
    println!("ciphertext: {:?}", ciphertext);
    // key.encrypt(&nonce, b"plaintext message".as_ref()).unwrap();

    println!("test");
    // curve25519
    // c25519::generate_secret_key(&mut private_key2);
    /*    let private_key2 = [2u8; KEY_LENGTH];
        let nonce = [0u8; NONCE_LENGTH];
        let message = [0u8; 131];

        let mut scsk = Sc25519 { v: [0; 32] };
        let mut gepk = make_ge25519();

        hash_sha512(&mut az, &seed);
        az[0] &= 248;
        az[31] &= 127;
        az[31] |= 64;

        sc25519_from32bytes(&mut scsk, &az[0..32]);*/
}

