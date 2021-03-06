use crate::secp256k1;

#[derive(Clone, Debug)]
pub struct Ciphertext {
    sk: secp256k1::SecretKey,
}

#[derive(Default)]
pub struct SecretKey;

#[derive(Default)]
pub struct PublicKey;

pub fn keygen() -> (SecretKey, PublicKey) {
    (SecretKey, PublicKey)
}

pub trait Encrypt {
    fn encrypt(&self, x: &secp256k1::KeyPair, witness: &secp256k1::SecretKey) -> Ciphertext;
}

impl Encrypt for SecretKey {
    fn encrypt(&self, _x: &secp256k1::KeyPair, witness: &secp256k1::SecretKey) -> Ciphertext {
        Ciphertext {
            sk: witness.clone(),
        }
    }
}

pub trait Pow<T> {
    fn pow(&self, t: &T, x: &secp256k1::KeyPair) -> T;
}

impl Pow<Ciphertext> for PublicKey {
    fn pow(&self, t: &Ciphertext, _x: &secp256k1::KeyPair) -> Ciphertext {
        t.clone()
    }
}

impl Pow<secp256k1::PublicKey> for PublicKey {
    fn pow(&self, t: &secp256k1::PublicKey, _x: &secp256k1::KeyPair) -> secp256k1::PublicKey {
        t.clone()
    }
}

pub trait Decrypt {
    fn decrypt(&self, x: &secp256k1::KeyPair, c: &Ciphertext) -> secp256k1::SecretKey;
}

impl Decrypt for SecretKey {
    fn decrypt(&self, _x: &secp256k1::KeyPair, c: &Ciphertext) -> secp256k1::SecretKey {
        c.sk.clone()
    }
}
