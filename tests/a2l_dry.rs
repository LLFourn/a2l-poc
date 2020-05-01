use a2l_poc::bitcoin::random_p2wpkh;
use a2l_poc::puzzle_promise;
use a2l_poc::puzzle_solver;
use a2l_poc::{hsm_cl, Params};
use serde::Serialize;

#[test]
fn dry_happy_path() {
    let mut blockchain = Blockchain::default();
    let mut bandwidth_recorder = BandwidthRecorder::default();
    let amount = 10_000_000;

    run_a2l_happy_path(amount, 0, 0, &mut blockchain, &mut bandwidth_recorder);

    assert!(blockchain.sender_fund.is_some());
    assert!(blockchain.tumbler_redeem.is_some());
    assert!(blockchain.tumbler_fund.is_some());
    assert!(blockchain.receiver_redeem.is_some());
}

#[test]
fn happy_path_fees() {
    let mut blockchain = Blockchain::default();
    let mut bandwidth_recorder = BandwidthRecorder::default();

    // global parameters
    let tumble_amount = 10_000_000;
    let tumbler_fee = 10_000;
    let spend_transaction_fee_per_wu = 15;

    run_a2l_happy_path(
        tumble_amount,
        tumbler_fee,
        spend_transaction_fee_per_wu,
        &mut blockchain,
        &mut bandwidth_recorder,
    );

    let (sender_fund, tumbler_redeem, tumbler_fund, receiver_redeem) = (
        blockchain.sender_fund.unwrap(),
        blockchain.tumbler_redeem.unwrap(),
        blockchain.tumbler_fund.unwrap(),
        blockchain.receiver_redeem.unwrap(),
    );

    assert_eq!(
        sender_fund.output[0].value,
        tumble_amount
            + tumbler_fee
            + a2l_poc::bitcoin::MAX_SATISFACTION_WEIGHT * spend_transaction_fee_per_wu
    );
    assert_eq!(tumbler_redeem.output[0].value, tumble_amount + tumbler_fee);
    assert_eq!(
        tumbler_fund.output[0].value,
        tumble_amount + a2l_poc::bitcoin::MAX_SATISFACTION_WEIGHT * spend_transaction_fee_per_wu
    );
    assert_eq!(receiver_redeem.output[0].value, tumble_amount);
}

#[test]
fn redeem_transaction_size() {
    let mut blockchain = Blockchain::default();
    let mut bandwidth_recorder = BandwidthRecorder::default();

    let amount = 10_000_000;
    run_a2l_happy_path(amount, 0, 0, &mut blockchain, &mut bandwidth_recorder);

    let (tumbler_redeem, receiver_redeem) = (
        blockchain.tumbler_redeem.unwrap(),
        blockchain.receiver_redeem.unwrap(),
    );

    let redeem_tx_weight = tumbler_redeem.get_weight() + receiver_redeem.get_weight();
    let max_expected_weight = 1095;

    assert!(max_expected_weight >= redeem_tx_weight);
}

#[test]
fn protocol_bandwidth() {
    let mut blockchain = Blockchain::default();
    let mut bandwidth_recorder = BandwidthRecorder::default();
    let amount = 10_000_000;

    run_a2l_happy_path(amount, 0, 0, &mut blockchain, &mut bandwidth_recorder);

    let total_bandwidth = bandwidth_recorder.bandwidth_used;
    let max_expected_bandwidth = 7146;

    assert!(
        max_expected_bandwidth >= total_bandwidth,
        "{} >= {}",
        max_expected_bandwidth,
        total_bandwidth
    );
}

#[derive(Default)]
struct BandwidthRecorder {
    bandwidth_used: usize,
}

impl BandwidthRecorder {
    fn record<M>(&mut self, message: M) -> M
    where
        M: Serialize,
    {
        let bytes = serde_cbor::to_vec(&message).expect("message to be serializable");

        self.bandwidth_used += bytes.len();

        message
    }
}

fn run_a2l_happy_path(
    tumble_amount: u64,
    tumbler_fee: u64,
    spend_transaction_fee_per_wu: u64,
    blockchain: &mut Blockchain,
    bandwidth_recorder: &mut BandwidthRecorder,
) {
    let mut rng = rand::thread_rng();

    let he_keypair = hsm_cl::keygen(b"A2L-PoC");
    let he_public_key = he_keypair.to_pk();

    let params = Params::new(
        random_p2wpkh(),
        random_p2wpkh(),
        0,
        tumble_amount,
        0,
        spend_transaction_fee_per_wu,
        bitcoin::Transaction {
            lock_time: 0,
            version: 2,
            input: Vec::new(),
            output: vec![bitcoin::TxOut {
                value: tumble_amount
                    + a2l_poc::bitcoin::MAX_SATISFACTION_WEIGHT * spend_transaction_fee_per_wu,
                script_pubkey: Default::default(),
            }],
        },
    );

    // puzzle promise protocol
    let tumbler = puzzle_promise::Tumbler0::new(params.clone(), &mut rng, he_keypair.clone());
    let receiver = puzzle_promise::Receiver0::new(params, &mut rng, he_public_key);
    let sender = puzzle_promise::Sender0::new();

    let message = tumbler.next_message();
    let receiver = receiver
        .receive(bandwidth_recorder.record(message))
        .unwrap();
    let message = receiver.next_message();
    let tumbler = tumbler.receive(bandwidth_recorder.record(message)).unwrap();
    let message = tumbler.next_message(&mut rng);
    let receiver = receiver
        .receive(bandwidth_recorder.record(message), &mut rng)
        .unwrap();
    let message = receiver.next_message();
    let sender = sender.receive(bandwidth_recorder.record(message));

    blockchain.tumbler_fund = Some(tumbler.unsigned_fund_transaction().clone());

    // puzzle solver protocol
    let params = Params::new(
        random_p2wpkh(),
        random_p2wpkh(),
        0,
        tumble_amount,
        tumbler_fee,
        spend_transaction_fee_per_wu,
        bitcoin::Transaction {
            lock_time: 0,
            version: 2,
            input: Vec::new(),
            output: vec![bitcoin::TxOut {
                value: tumble_amount
                    + tumbler_fee
                    + a2l_poc::bitcoin::MAX_SATISFACTION_WEIGHT * spend_transaction_fee_per_wu,
                script_pubkey: Default::default(),
            }],
        },
    );

    let tumbler = puzzle_solver::Tumbler0::new(params.clone(), he_keypair, &mut rng);
    let sender = puzzle_solver::Sender0::new(params, sender.lock().clone(), &mut rng);
    let receiver = puzzle_solver::Receiver0::new(
        receiver.x_r().to_pk(),
        receiver.X_t().clone(),
        receiver.unsigned_redeem_transaction().clone(),
        receiver.sig_redeem_t().clone(),
        receiver.sig_redeem_r().clone(),
        receiver.beta().clone(),
        *receiver.redeem_tx_digest(),
    );

    let message = tumbler.next_message();
    let sender = sender.receive(bandwidth_recorder.record(message), &mut rng);
    let message = sender.next_message();
    let tumbler = tumbler.receive(bandwidth_recorder.record(message));
    let message = tumbler.next_message();
    let sender = sender
        .receive(bandwidth_recorder.record(message), &mut rng)
        .unwrap();
    let message = sender.next_message();
    let tumbler = tumbler.receive(bandwidth_recorder.record(message)).unwrap();

    blockchain.sender_fund = Some(sender.unsigned_fund_transaction());
    blockchain.tumbler_redeem = Some(tumbler.signed_redeem_transaction().clone());

    let sender = sender
        .receive(blockchain.tumbler_redeem.clone().unwrap())
        .unwrap();
    let message = sender.next_message();
    let receiver = receiver
        .receive(bandwidth_recorder.record(message))
        .unwrap();

    blockchain.receiver_redeem = Some(receiver.signed_redeem_transaction().clone());
}

#[derive(Default)]
struct Blockchain {
    pub sender_fund: Option<bitcoin::Transaction>,
    pub tumbler_redeem: Option<bitcoin::Transaction>,
    pub sender_refund: Option<bitcoin::Transaction>,
    pub tumbler_fund: Option<bitcoin::Transaction>,
    pub receiver_redeem: Option<bitcoin::Transaction>,
    pub tumbler_refund: Option<bitcoin::Transaction>,
}
