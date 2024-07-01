use lazy_static::lazy_static;
use pessimistic_proof::{
    certificate::Certificate,
    generate_leaf_proof,
    local_balance_tree::{BalanceTree, Deposit},
    local_exit_tree::{hasher::Keccak256Hasher, LocalExitTree},
    BridgeExit, LocalNetworkState, NetworkId, ProofError, TokenInfo,
};
use reth_primitives::{address, U256};
use rstest::rstest;

lazy_static! {
    pub static ref NETWORK_A: NetworkId = 0.into();
    pub static ref NETWORK_B: NetworkId = 1.into();
    pub static ref USDC: TokenInfo = TokenInfo {
        origin_network: *NETWORK_A,
        origin_token_address: address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
    };
    pub static ref ETH: TokenInfo = TokenInfo {
        origin_network: *NETWORK_A,
        origin_token_address: address!("0000000000000000000000000000000000000000"),
    };
    pub static ref DUMMY_LET: LocalExitTree<Keccak256Hasher> =
        LocalExitTree::from_leaves([[0_u8; 32], [1_u8; 32], [2_u8; 32]].into_iter());
}

struct Amounts {
    eth: u32,
    usdc: u32,
}

impl From<Amounts> for BalanceTree {
    fn from(v: Amounts) -> Self {
        let eth = (ETH.clone(), Deposit(U256::from(v.eth)).into());
        let usdc = (USDC.clone(), Deposit(U256::from(v.usdc)).into());
        BalanceTree::from(vec![eth, usdc])
    }
}

fn make_tx(_from: u32, to: u32, token: &TokenInfo, amount: u32) -> BridgeExit {
    BridgeExit::new(
        0,
        token.origin_network,
        token.origin_token_address,
        to.into(),
        address!("a8da6bf26964af9d7eed9e03e53415d37aa96045"),
        U256::from(amount),
        Vec::new(),
    )
}

fn state_transition(v: Amounts) -> Certificate {
    Certificate::new(
        *NETWORK_A,
        DUMMY_LET.get_root(),
        vec![make_tx(0, 1, &ETH, v.eth), make_tx(0, 1, &USDC, v.usdc)],
    )
}

fn initial_state(amount: Amounts) -> LocalNetworkState {
    LocalNetworkState {
        exit_tree: DUMMY_LET.clone(),
        balance_tree: amount.into(),
        nullifier_tree: Default::default(),
    }
}

#[rstest]
fn should_succeed() {
    let state = initial_state(Amounts { eth: 10, usdc: 100 });

    let cert = state_transition(Amounts { eth: 10, usdc: 100 });
    assert!(generate_leaf_proof(state.clone(), cert).is_ok());

    let cert = state_transition(Amounts { eth: 0, usdc: 0 });
    assert!(generate_leaf_proof(state.clone(), cert).is_ok());

    let cert = state_transition(Amounts { eth: 10, usdc: 99 });
    assert!(generate_leaf_proof(state.clone(), cert).is_ok());
}

#[rstest]
fn should_detect_debtor() {
    let state = initial_state(Amounts { eth: 10, usdc: 100 });

    let cert = state_transition(Amounts { eth: 10, usdc: 101 });
    assert!(matches!(
        generate_leaf_proof(state.clone(), cert),
        Err(ProofError::HasDebt { .. })
    ));

    let cert = state_transition(Amounts { eth: 20, usdc: 200 });
    assert!(matches!(
        generate_leaf_proof(state.clone(), cert),
        Err(ProofError::HasDebt { .. })
    ));

    let cert = state_transition(Amounts { eth: 0, usdc: 200 });
    assert!(matches!(
        generate_leaf_proof(state.clone(), cert),
        Err(ProofError::HasDebt { .. })
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn test_full_proof_mainnet_data() {
    // from data fetched from mainnet
}
