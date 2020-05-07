use crate::Lock;
use crate::{bitcoin, hsm_cl, secp256k1, NoTransaction, Params, UnexpectedMessage};
use anyhow::Context;
use rand::Rng;

#[derive(Debug, derive_more::From, serde::Serialize)]
pub enum Message {
    Message0(Message0),
    Message1(Message1),
    Message2(Message2),
    Message3(Message3),
}

#[derive(Debug, serde::Serialize)]
pub struct Message0 {
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub X_t: secp256k1::PublicKey,
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub A: secp256k1::PublicKey,
    pub c_alpha: hsm_cl::Ciphertext,
    pub pi_alpha: hsm_cl::Proof,
}

#[derive(Debug, serde::Serialize)]
pub struct Message1 {
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub X_r: secp256k1::PublicKey,
    #[serde(with = "crate::serde::secp256k1_signature")]
    pub sig_refund_r: secp256k1::Signature,
}

#[derive(Debug, serde::Serialize)]
pub struct Message2 {
    pub sig_redeem_t: secp256k1::EncryptedSignature,
}

#[derive(Debug, serde::Serialize)]
pub struct Message3 {
    pub l: Lock,
}

#[derive(Debug, derive_more::From)]
pub enum Tumbler {
    Tumbler0(Tumbler0),
    Tumbler1(Tumbler1),
}

impl Tumbler {
    pub fn new(params: Params, HE: hsm_cl::KeyPair, rng: &mut impl Rng) -> Self {
        let tumbler = Tumbler0::new(params, HE, rng);

        tumbler.into()
    }

    pub fn transition(self, message: Message, rng: &mut impl Rng) -> anyhow::Result<Self> {
        let tumbler = match (self, message) {
            (Tumbler::Tumbler0(inner), Message::Message1(message)) => {
                inner.receive(message, rng)?.into()
            }
            _ => anyhow::bail!(UnexpectedMessage),
        };

        Ok(tumbler)
    }

    pub fn next_message(&self) -> Message {
        match self {
            Tumbler::Tumbler0(inner) => inner.next_message().into(),
            Tumbler::Tumbler1(inner) => inner.next_message().into(),
        }
    }

    pub fn fund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        let transaction = match self {
            Tumbler::Tumbler1(inner) => inner.unsigned_fund_transaction().clone(),
            _ => anyhow::bail!(NoTransaction),
        };

        Ok(transaction)
    }

    pub fn refund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        let transaction = match self {
            Tumbler::Tumbler1(inner) => inner.signed_refund_transaction().clone(),
            _ => anyhow::bail!(NoTransaction),
        };

        Ok(transaction)
    }
}

#[derive(Debug)]
pub struct Tumbler0 {
    x_t: secp256k1::KeyPair,
    a: secp256k1::KeyPair,
    params: Params,
    HE: hsm_cl::KeyPair,
}

#[derive(Debug)]
pub struct Tumbler1 {
    x_t: secp256k1::KeyPair,
    a: secp256k1::KeyPair,
    signed_refund_transaction: bitcoin::Transaction,
    transactions: bitcoin::Transactions,
    sig_redeem_t: secp256k1::EncryptedSignature,
}

impl Tumbler0 {
    pub fn new(params: Params, HE: hsm_cl::KeyPair, rng: &mut impl Rng) -> Self {
        let x_t = secp256k1::KeyPair::random(rng);
        let a = secp256k1::KeyPair::random(rng);

        Self { x_t, a, params, HE }
    }

    pub fn next_message(&self) -> Message0 {
        let X_t = self.x_t.to_pk();
        let A = self.a.to_pk();
        let (c_alpha, pi_alpha) = hsm_cl::encrypt(&self.HE.to_pk(), &self.a);

        Message0 {
            X_t,
            A,
            c_alpha,
            pi_alpha,
        }
    }

    pub fn receive(
        self,
        Message1 { X_r, sig_refund_r }: Message1,
        rng: &mut impl Rng,
    ) -> anyhow::Result<Tumbler1> {
        let transactions = bitcoin::make_transactions(
            self.params.partial_fund_transaction.clone(),
            self.params.tumbler_receiver_joint_output_value(),
            self.params.tumbler_receiver_joint_output_takeout(),
            &self.x_t.to_pk(),
            &X_r,
            self.params.expiry,
            &self.params.redeem_identity,
            &self.params.refund_identity,
        );

        let signed_refund_transaction = {
            secp256k1::verify(transactions.refund_tx_digest, &sig_refund_r, &X_r)
                .context("failed to verify receiver refund signature")?;

            let sig_refund_t = secp256k1::sign(transactions.refund_tx_digest, &self.x_t);

            bitcoin::complete_spend_transaction(
                transactions.refund.clone(),
                (self.x_t.to_pk(), sig_refund_t),
                (X_r, sig_refund_r),
            )?
        };

        let sig_redeem_t = secp256k1::encsign(
            transactions.redeem_tx_digest,
            &self.x_t,
            &self.a.to_pk(),
            rng,
        );

        Ok(Tumbler1 {
            x_t: self.x_t,
            signed_refund_transaction,
            a: self.a,
            transactions,
            sig_redeem_t,
        })
    }
}

impl Tumbler1 {
    pub fn next_message(&self) -> Message2 {
        Message2 {
            sig_redeem_t: self.sig_redeem_t.clone(),
        }
    }

    pub fn unsigned_fund_transaction(&self) -> &bitcoin::Transaction {
        &self.transactions.fund
    }
    pub fn signed_refund_transaction(&self) -> &bitcoin::Transaction {
        &self.signed_refund_transaction
    }
    pub fn x_t(&self) -> &secp256k1::KeyPair {
        &self.x_t
    }
}
