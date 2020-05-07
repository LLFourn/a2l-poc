use crate::{bitcoin, hsm_cl, secp256k1, NoMessage, NoTransaction, Params, UnexpectedMessage};
use rand::Rng;

#[derive(Debug, derive_more::From, serde::Serialize)]
pub enum Message {
    Message0(Message0),
    Message1(Message1),
    Message2(Message2),
    Message3(Message3),
    Message4(Message4),
}

#[derive(Debug, serde::Serialize)]
pub struct Message0 {
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub X_t: secp256k1::PublicKey,
}

#[derive(Debug, serde::Serialize)]
pub struct Message1 {
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub X_s: secp256k1::PublicKey,
    pub c_alpha_prime_prime: hsm_cl::Ciphertext,
}

#[derive(Debug, serde::Serialize)]
pub struct Message2 {
    #[serde(with = "crate::serde::secp256k1_public_key")]
    pub A_prime_prime: secp256k1::PublicKey,
    #[serde(with = "crate::serde::secp256k1_signature")]
    pub sig_refund_t: secp256k1::Signature,
}

#[derive(Debug, serde::Serialize)]
pub struct Message3 {
    pub sig_redeem_s: secp256k1::EncryptedSignature,
}

#[derive(Debug, serde::Serialize)]
pub struct Message4 {
    #[serde(with = "crate::serde::secp256k1_secret_key")]
    pub alpha_macron: secp256k1::SecretKey,
}

#[derive(Debug, derive_more::From, Clone)]
pub enum Tumbler {
    Tumbler0(Tumbler0),
    Tumbler1(Tumbler1),
    Tumbler2(Tumbler2),
}

impl Tumbler {
    pub fn new(params: Params, HE: hsm_cl::KeyPair, rng: &mut impl Rng) -> Self {
        let tumbler = Tumbler0::new(params, HE, rng);

        tumbler.into()
    }

    pub fn transition(self, message: Message) -> anyhow::Result<Self> {
        let tumbler = match (self, message) {
            (Tumbler::Tumbler0(inner), Message::Message1(message)) => inner.receive(message).into(),
            (Tumbler::Tumbler1(inner), Message::Message3(message)) => {
                inner.receive(message)?.into()
            }
            _ => anyhow::bail!(UnexpectedMessage),
        };

        Ok(tumbler)
    }

    pub fn next_message(&self) -> anyhow::Result<Message> {
        let message = match self {
            Tumbler::Tumbler0(inner) => inner.next_message().into(),
            Tumbler::Tumbler1(inner) => inner.next_message().into(),
            _ => anyhow::bail!(NoMessage),
        };

        Ok(message)
    }

    pub fn redeem_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        let transaction = match self {
            Tumbler::Tumbler2(inner) => inner.signed_redeem_transaction.clone(),
            _ => anyhow::bail!(NoTransaction),
        };

        Ok(transaction)
    }
}

#[derive(Debug, Clone)]
pub struct Tumbler0 {
    x_t: secp256k1::KeyPair,
    params: Params,
    HE: hsm_cl::KeyPair,
}

#[derive(Debug, Clone)]
pub struct Tumbler1 {
    transactions: bitcoin::Transactions,
    x_t: secp256k1::KeyPair,
    X_s: secp256k1::PublicKey,
    gamma: secp256k1::KeyPair,
    sig_refund_t: secp256k1::Signature,
}

#[derive(Debug, Clone)]
pub struct Tumbler2 {
    signed_redeem_transaction: bitcoin::Transaction,
}

impl Tumbler0 {
    pub fn new(params: Params, HE: hsm_cl::KeyPair, rng: &mut impl Rng) -> Self {
        Self {
            params,
            x_t: secp256k1::KeyPair::random(rng),
            HE,
        }
    }

    pub fn next_message(&self) -> Message0 {
        Message0 {
            X_t: self.x_t.to_pk(),
        }
    }

    pub fn receive(
        self,
        Message1 {
            X_s,
            c_alpha_prime_prime,
        }: Message1,
    ) -> Tumbler1 {
        let gamma = hsm_cl::decrypt(&self.HE, &c_alpha_prime_prime).into();

        let transactions = bitcoin::make_transactions(
            self.params.partial_fund_transaction.clone(),
            self.params.sender_tumbler_joint_output_value(),
            self.params.sender_tumbler_joint_output_takeout(),
            &X_s,
            &self.x_t.to_pk(),
            self.params.expiry,
            &self.params.redeem_identity,
            &self.params.refund_identity,
        );

        let sig_refund_t = secp256k1::sign(transactions.refund_tx_digest, &self.x_t);

        Tumbler1 {
            transactions,
            x_t: self.x_t,
            X_s,
            gamma,
            sig_refund_t,
        }
    }
}

impl Tumbler1 {
    pub fn next_message(&self) -> Message2 {
        Message2 {
            A_prime_prime: self.gamma.to_pk(),
            sig_refund_t: self.sig_refund_t.clone(),
        }
    }

    pub fn receive(self, Message3 { sig_redeem_s }: Message3) -> anyhow::Result<Tumbler2> {
        let Self {
            transactions,
            x_t,
            X_s,
            gamma,
            ..
        } = self;

        let signed_redeem_transaction = {
            let sig_redeem_s = secp256k1::decsig(&gamma, &sig_redeem_s);
            secp256k1::verify(transactions.redeem_tx_digest, &sig_redeem_s, &X_s)?;

            let sig_redeem_t = secp256k1::sign(transactions.redeem_tx_digest, &x_t);

            bitcoin::complete_spend_transaction(
                transactions.redeem,
                (X_s, sig_redeem_s),
                (x_t.to_pk(), sig_redeem_t),
            )?
        };

        Ok(Tumbler2 {
            signed_redeem_transaction,
        })
    }
}

impl Tumbler2 {
    pub fn signed_redeem_transaction(&self) -> &bitcoin::Transaction {
        &self.signed_redeem_transaction
    }
}
