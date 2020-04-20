use crate::bitcoin;
use crate::dummy_hsm_cl as hsm_cl;
use crate::secp256k1;
use crate::Params;
use ::bitcoin::hashes::Hash;
use anyhow::Context;
use fehler::throws;

pub struct Tumbler0 {
    x_t: secp256k1::KeyPair,
    a: secp256k1::KeyPair,
    pi_alpha: hsm_cl::Proof,
    l: hsm_cl::Puzzle,
    params: Params,
}

pub struct Sender0;

pub struct Receiver0 {
    x_r: secp256k1::KeyPair,
    params: Params,
    hsm_cl: hsm_cl::System<hsm_cl::PublicKey>,
}

pub struct Sender1 {
    l_prime: hsm_cl::Puzzle,
}

pub struct Tumbler1 {
    X_r: secp256k1::PublicKey,
    x_t: secp256k1::KeyPair,
    a: secp256k1::KeyPair,
    sig_refund_t: secp256k1::Signature,
    sig_refund_r: secp256k1::Signature,
    fund_transaction: bitcoin::Transaction,
    refund_transaction: bitcoin::Transaction,
    redeem_amount: u64,
    redeem_identity: secp256k1::PublicKey,
}

pub struct Receiver1 {
    x_r: secp256k1::KeyPair,
    X_t: secp256k1::PublicKey,
    hsm_cl: hsm_cl::System<hsm_cl::PublicKey>,
    l: hsm_cl::Puzzle,
    fund_transaction: bitcoin::Transaction,
    redeem_identity: secp256k1::PublicKey,
    refund_identity: secp256k1::PublicKey,
    expiry: u32,
    amount: u64,
}

pub struct Receiver2 {
    beta: secp256k1::KeyPair,
    l_prime: hsm_cl::Puzzle,
    redeem_transaction: bitcoin::Transaction,
    sig_redeem_r: secp256k1::Signature,
    sig_redeem_t: secp256k1::EncryptedSignature,
}

impl Receiver0 {
    pub fn new<R: rand::Rng>(params: Params, hsm_cl: hsm_cl::PublicKey, rng: &mut R) -> Self {
        Self {
            x_r: secp256k1::KeyPair::random(rng),
            params,
            hsm_cl: hsm_cl::System::new(hsm_cl),
        }
    }

    #[throws(anyhow::Error)]
    pub fn receive(self, Message0 { X_t, pi_alpha, l }: Message0) -> Receiver1 {
        let Receiver0 {
            x_r,
            params:
                Params {
                    redeem_identity,
                    refund_identity,
                    expiry,
                    partial_fund_transaction: fund_transaction,
                    amount,
                },
            hsm_cl,
        } = self;

        hsm_cl.verify_puzzle(pi_alpha, &l)?;

        // TODO: account for fee in these amounts

        let fund_transaction =
            bitcoin::make_fund_transaction(fund_transaction, amount, &X_t, &x_r.to_pk());

        Receiver1 {
            x_r,
            X_t,
            fund_transaction,
            hsm_cl,
            l,
            redeem_identity,
            refund_identity,
            expiry,
            amount,
        }
    }
}

impl Receiver1 {
    pub fn next_message(&self) -> Message1 {
        let (_, digest) = bitcoin::make_spend_transaction(
            &self.fund_transaction,
            self.amount,
            &self.refund_identity,
            self.expiry,
        );
        let sig_refund_r = secp256k1::sign(digest, &self.x_r);

        Message1 {
            X_r: self.x_r.to_pk(),
            sig_refund_r,
        }
    }

    #[throws(anyhow::Error)]
    pub fn receive<R: rand::Rng>(
        self,
        Message2 { sig_redeem_t }: Message2,
        rng: &mut R,
    ) -> Receiver2 {
        let (redeem_transaction, digest) = bitcoin::make_spend_transaction(
            &self.fund_transaction,
            self.amount,
            &self.redeem_identity,
            0,
        );

        secp256k1::encverify(&self.X_t, &self.l.A, &digest.into_inner(), &sig_redeem_t)?;

        let sig_redeem_r = secp256k1::sign(digest, &self.x_r);

        let beta = secp256k1::KeyPair::random(rng);
        let l_prime = self.hsm_cl.randomize_puzzle(&self.l, &beta);

        Receiver2 {
            beta,
            l_prime,
            sig_redeem_r,
            sig_redeem_t,
            redeem_transaction,
        }
    }
}

impl Tumbler0 {
    pub fn new<R: rand::Rng>(params: Params, hsm_cl: hsm_cl::SecretKey, rng: &mut R) -> Self {
        let hsm_cl = hsm_cl::System::new(hsm_cl);

        let x_t = secp256k1::KeyPair::random(rng);
        let a = secp256k1::KeyPair::random(rng);
        let (pi_alpha, l) = hsm_cl.make_puzzle(&x_t, &a);

        Self {
            x_t,
            a,
            l,
            pi_alpha,
            params,
        }
    }

    pub fn next_message(&self) -> Message0 {
        Message0 {
            X_t: self.x_t.to_pk(),
            l: self.l.clone(),
            pi_alpha: self.pi_alpha.clone(),
        }
    }

    #[throws(anyhow::Error)]
    pub fn receive(self, Message1 { X_r, sig_refund_r }: Message1) -> Tumbler1 {
        let fund_transaction = bitcoin::make_fund_transaction(
            self.params.partial_fund_transaction,
            self.params.amount,
            &self.x_t.to_pk(),
            &X_r,
        );

        let (refund_transaction, sig_refund_t) = {
            let (transaction, digest) = bitcoin::make_spend_transaction(
                &fund_transaction,
                self.params.amount,
                &self.params.refund_identity,
                self.params.expiry,
            );

            secp256k1::verify(digest, &sig_refund_r, &X_r)
                .context("failed to verify receiver refund signature")?;

            let signature = secp256k1::sign(digest, &self.x_t);

            (transaction, signature)
        };

        Tumbler1 {
            X_r,
            x_t: self.x_t,
            sig_refund_t,
            sig_refund_r,
            fund_transaction,
            refund_transaction,
            a: self.a,
            redeem_amount: self.params.amount, // TODO: Handle fee
            redeem_identity: self.params.redeem_identity,
        }
    }
}

impl Tumbler1 {
    pub fn next_message<R: rand::Rng>(&self, rng: &mut R) -> Message2 {
        let (_, digest) = bitcoin::make_spend_transaction(
            &self.fund_transaction,
            self.redeem_amount,
            &self.redeem_identity,
            0,
        );
        let sig_redeem_t = secp256k1::encsign(digest, &self.x_t, &self.a.to_pk(), rng);

        Message2 { sig_redeem_t }
    }

    #[throws(anyhow::Error)]
    pub fn output(self) -> TumblerOutput {
        TumblerOutput {
            unsigned_fund_transaction: self.fund_transaction,
            signed_refund_transaction: bitcoin::complete_spend_transaction(
                self.refund_transaction,
                (self.x_t.to_pk(), self.sig_refund_t),
                (self.X_r, self.sig_refund_r),
            )?,
        }
    }
}

#[derive(Debug)]
pub struct TumblerOutput {
    unsigned_fund_transaction: bitcoin::Transaction,
    signed_refund_transaction: bitcoin::Transaction,
}

#[derive(Debug)]
pub struct ReceiverOutput {
    unsigned_redeem_transaction: bitcoin::Transaction,
    sig_redeem_t: secp256k1::EncryptedSignature,
    sig_redeem_r: secp256k1::Signature,
    beta: secp256k1::KeyPair,
}

#[derive(Debug)]
pub struct SenderOutput {
    l_prime: hsm_cl::Puzzle,
}

impl Receiver2 {
    pub fn next_message(&self) -> Message3 {
        Message3 {
            l_prime: self.l_prime.clone(),
        }
    }

    pub fn output(self) -> ReceiverOutput {
        ReceiverOutput {
            unsigned_redeem_transaction: self.redeem_transaction,
            sig_redeem_t: self.sig_redeem_t,
            sig_redeem_r: self.sig_redeem_r,
            beta: self.beta,
        }
    }
}

impl Sender0 {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }

    pub fn receive(self, message: Message3) -> Sender1 {
        Sender1 {
            l_prime: message.l_prime,
        }
    }
}

impl Sender1 {
    pub fn output(self) -> SenderOutput {
        SenderOutput {
            l_prime: self.l_prime,
        }
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
    sig_refund_r: secp256k1::Signature,
}

pub struct Message2 {
    sig_redeem_t: secp256k1::EncryptedSignature,
}

// receiver to sender
pub struct Message3 {
    l_prime: hsm_cl::Puzzle,
}
