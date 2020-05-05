mod run_happy_path;
mod run_refund;

pub use self::run_happy_path::run_happy_path;
pub use self::run_refund::run_refund;
use a2l::{puzzle_promise, puzzle_solver, receiver::Receiver, sender::Sender};
use rand::Rng;

pub trait Transition<M>: Sized {
    fn transition(self, message: M, rng: &mut impl Rng) -> anyhow::Result<Self>;
}

pub trait NextMessage<M> {
    fn next_message(&self, rng: &mut impl Rng) -> anyhow::Result<M>;
}

pub trait FundTransaction {
    fn fund_transaction(&self) -> anyhow::Result<bitcoin::Transaction>;
}

pub trait RedeemTransaction {
    fn redeem_transaction(&self) -> anyhow::Result<bitcoin::Transaction>;
}

pub trait RefundTransaction {
    fn refund_transaction(&self) -> anyhow::Result<bitcoin::Transaction>;
}

impl Transition<puzzle_promise::Message> for puzzle_promise::Tumbler {
    fn transition(
        self,
        message: puzzle_promise::Message,
        _: &mut impl Rng,
    ) -> anyhow::Result<Self> {
        self.transition(message)
    }
}

impl NextMessage<puzzle_promise::Message> for puzzle_promise::Tumbler {
    fn next_message(&self, rng: &mut impl Rng) -> anyhow::Result<puzzle_promise::Message> {
        Ok(self.next_message(rng))
    }
}

impl FundTransaction for puzzle_promise::Tumbler {
    fn fund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        self.fund_transaction()
    }
}

impl RefundTransaction for puzzle_promise::Tumbler {
    fn refund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        self.refund_transaction()
    }
}

impl Transition<puzzle_solver::Message> for puzzle_solver::Tumbler {
    fn transition(self, message: puzzle_solver::Message, _: &mut impl Rng) -> anyhow::Result<Self> {
        self.transition(message)
    }
}

impl NextMessage<puzzle_solver::Message> for puzzle_solver::Tumbler {
    fn next_message(&self, _: &mut impl Rng) -> anyhow::Result<puzzle_solver::Message> {
        self.next_message()
    }
}

impl RedeemTransaction for puzzle_solver::Tumbler {
    fn redeem_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        self.redeem_transaction()
    }
}

impl Transition<puzzle_promise::Message> for Receiver {
    fn transition(
        self,
        message: puzzle_promise::Message,
        rng: &mut impl Rng,
    ) -> anyhow::Result<Self> {
        self.transition_on_puzzle_promise_message(message, rng)
    }
}

impl Transition<puzzle_solver::Message> for Receiver {
    fn transition(self, message: puzzle_solver::Message, _: &mut impl Rng) -> anyhow::Result<Self> {
        self.transition_on_puzzle_solver_message(message)
    }
}

impl NextMessage<puzzle_promise::Message> for Receiver {
    fn next_message(&self, _: &mut impl Rng) -> anyhow::Result<puzzle_promise::Message> {
        self.next_puzzle_promise_message()
    }
}

impl RedeemTransaction for Receiver {
    fn redeem_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        self.redeem_transaction()
    }
}

impl Transition<puzzle_promise::Message> for Sender {
    fn transition(
        self,
        message: puzzle_promise::Message,
        _: &mut impl Rng,
    ) -> anyhow::Result<Self> {
        self.transition_on_puzzle_promise_message(message)
    }
}

impl Transition<puzzle_solver::Message> for Sender {
    fn transition(
        self,
        message: puzzle_solver::Message,
        rng: &mut impl Rng,
    ) -> anyhow::Result<Self> {
        self.transition_on_puzzle_solver_message(message, rng)
    }
}

impl Transition<bitcoin::Transaction> for Sender {
    fn transition(
        self,
        transaction: bitcoin::Transaction,
        _: &mut impl Rng,
    ) -> anyhow::Result<Self> {
        self.transition_on_transaction(transaction)
    }
}

impl FundTransaction for Sender {
    fn fund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        let transaction = self.fund_transaction()?;

        Ok(transaction)
    }
}

impl RefundTransaction for Sender {
    fn refund_transaction(&self) -> anyhow::Result<bitcoin::Transaction> {
        let transaction = self.refund_transaction()?;

        Ok(transaction)
    }
}

impl NextMessage<puzzle_solver::Message> for Sender {
    fn next_message(&self, _: &mut impl Rng) -> anyhow::Result<puzzle_solver::Message> {
        self.next_puzzle_solver_message()
    }
}

pub fn random_p2wpkh() -> ::bitcoin::Address {
    ::bitcoin::Address::p2wpkh(
        &::bitcoin::PublicKey::from_private_key(
            &::bitcoin::secp256k1::Secp256k1::signing_only(),
            &::bitcoin::PrivateKey {
                compressed: true,
                network: ::bitcoin::Network::Regtest,
                key: ::bitcoin::secp256k1::SecretKey::new(
                    &mut ::bitcoin::secp256k1::rand::thread_rng(),
                ),
            },
        ),
        ::bitcoin::Network::Regtest,
    )
}
