use crate::dummy_hsm_cl as hsm_cl;
use crate::*;
use std::rc::Rc;

pub struct Tumbler0 {
    x_t: secp256k1::KeyPair,
    hsm_cl: Rc<hsm_cl::System>,
    a: hsm_cl::KeyPair,
    pi_alpha: hsm_cl::Proof,
    l: hsm_cl::Puzzle,
}

pub struct Sender0;

pub struct Receiver0 {
    x_r: secp256k1::KeyPair,
    params: Params,
    hsm_cl: Rc<hsm_cl::System>,
}

pub struct Sender1;

pub struct Tumbler1;

pub struct Receiver1 {
    x_r: secp256k1::KeyPair,
    X_t: secp256k1::PublicKey,
    params: Params,
    hsm_cl: Rc<hsm_cl::System>,
    pi_alpha: hsm_cl::Proof,
    l: hsm_cl::Puzzle,
}

pub struct Receiver2 {
    hsm_cl: Rc<hsm_cl::System>,
    beta: hsm_cl::KeyPair,
    l_prime: hsm_cl::Puzzle,
}

impl Receiver0 {
    pub fn new(params: Params, x_r: secp256k1::KeyPair, hsm_cl: Rc<hsm_cl::System>) -> Self {
        Self {
            x_r,
            params,
            hsm_cl,
        }
    }

    pub fn receive(self, Message0 { X_t, pi_alpha, l }: Message0) -> Receiver1 {
        let Receiver0 {
            x_r,
            params,
            hsm_cl,
        } = self;

        Receiver1 {
            x_r,
            X_t,
            params,
            hsm_cl,
            pi_alpha,
            l,
        }
    }
}

impl Receiver1 {
    pub fn next_message(&self) -> Message1 {
        Message1 {
            X_r: self.x_r.to_pk(),
            // refund_sig: secp256k1::Signature,
        }
    }

    pub fn receive(self, _message: Message2) -> Receiver2 {
        let (beta, l_prime) = self.hsm_cl.randomize_puzzle(&self.l);

        Receiver2 {
            hsm_cl: self.hsm_cl,
            beta,
            l_prime,
        }
    }
}

impl Tumbler0 {
    pub fn new(_params: Params, x_t: secp256k1::KeyPair, hsm_cl: Rc<hsm_cl::System>) -> Self {
        let (a, pi_alpha, l) = hsm_cl.make_puzzle(&x_t);

        Self {
            x_t,
            hsm_cl,
            a,
            l,
            pi_alpha,
        }
    }

    pub fn next_message(&self) -> Message0 {
        Message0 {
            X_t: self.x_t.to_pk(),
            l: self.l.clone(),
            pi_alpha: self.pi_alpha.clone(),
        }
    }

    pub fn receive(self, _message: Message1) -> Tumbler1 {
        Tumbler1
    }
}

impl Tumbler1 {
    pub fn next_message(&self) -> Message2 {
        Message2::default()
    }
}

impl Receiver2 {
    pub fn next_message(&self) -> Message3 {
        Message3 {
            l_prime: self.l_prime.clone(),
        }
    }
}

impl Sender0 {
    pub fn new() -> Self {
        Self
    }

    pub fn receive(self, _message: Message3) -> Sender1 {
        Sender1
    }
}

pub struct Message0 {
    // key generation
    X_t: secp256k1::PublicKey,
    // protocol
    l: hsm_cl::Puzzle,
    pi_alpha: hsm_cl::Proof,
}

pub struct Message1 {
    // key generation
    X_r: secp256k1::PublicKey,
    // protocol
    // refund_sig: secp256k1::Signature,
}

#[derive(Default)]
pub struct Message2 {
    // redeem_encsig: EncryptedSignature,
}

// receiver to sender
pub struct Message3 {
    l_prime: hsm_cl::Puzzle,
}
