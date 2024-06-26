use std::collections::BTreeMap;

use lazy_static::lazy_static;
use pessimistic_proof::{
    certificate::Certificate,
    generate_leaf_proof,
    local_balance_tree::{Balance, BalanceTree, BalanceTreeByNetwork, Deposit},
    local_exit_tree::{hasher::Keccak256Hasher, LocalExitTree},
    BridgeExit, NetworkId, ProofError, TokenInfo,
};
use reth_primitives::{address, U256};
use rstest::{fixture, rstest};

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

#[fixture]
fn state_transition() -> Vec<Certificate> {
    let eth = ETH.clone();
    let usdc = USDC.clone();

    // Prepare the data fetched from the CDK: BridgeExits + LBT
    // BridgeExits
    let withdraw_0_to_1 = vec![make_tx(0, 1, &eth, 10), make_tx(0, 1, &usdc, 100)];
    let withdraw_1_to_0 = vec![make_tx(1, 0, &eth, 20), make_tx(1, 0, &usdc, 200)];

    let dummy_let: LocalExitTree<Keccak256Hasher> =
        LocalExitTree::from_leaves([[0_u8; 32], [1_u8; 32], [2_u8; 32]].into_iter());

    vec![
        Certificate::new(*NETWORK_A, dummy_let.get_root(), withdraw_0_to_1.clone()),
        Certificate::new(*NETWORK_B, dummy_let.get_root(), withdraw_1_to_0.clone()),
    ]
}

mod should_detect_debtor {
    use pessimistic_proof::LocalNetworkState;

    use super::*;

    #[fixture]
    fn initial_state() -> LocalNetworkState {
        let deposit_eth =
            |v: u32| -> (TokenInfo, Balance) { (ETH.clone(), Deposit(U256::from(v)).into()) };
        let deposit_usdc =
            |v: u32| -> (TokenInfo, Balance) { (USDC.clone(), Deposit(U256::from(v)).into()) };

        let dummy_let =
            LocalExitTree::from_leaves([[0_u8; 32], [1_u8; 32], [2_u8; 32]].into_iter());

        let initial_0 = BalanceTree::from(vec![deposit_eth(10), deposit_usdc(10)]);
        let initial_1 = BalanceTree::from(vec![deposit_eth(1), deposit_usdc(200)]);

        let global_balance_tree: BalanceTreeByNetwork = {
            let base: BTreeMap<NetworkId, BalanceTree> =
                [(*NETWORK_A, initial_0), (*NETWORK_B, initial_1)].into();
            base.into()
        };

        State {
            global_exit_tree: [(*NETWORK_A, dummy_let.clone()), (*NETWORK_B, dummy_let.clone())]
                .into(),
            global_balance_tree,
        }
    }

    #[rstest]
    fn prove(initial_state: State, state_transition: Vec<Certificate>) {
        assert!(matches!(
            generate_leaf_proof(initial_state, state_transition),
            Err(ProofError::HasDebt { .. })
        ));
    }
}

mod should_succeed {
    use super::*;

    #[fixture]
    fn initial_state() -> State {
        let deposit_eth =
            |v: u32| -> (TokenInfo, Balance) { (ETH.clone(), Deposit(U256::from(v)).into()) };
        let deposit_usdc =
            |v: u32| -> (TokenInfo, Balance) { (USDC.clone(), Deposit(U256::from(v)).into()) };

        let dummy_let =
            LocalExitTree::from_leaves([[0_u8; 32], [1_u8; 32], [2_u8; 32]].into_iter());

        let initial_0 = BalanceTree::from(vec![deposit_eth(12), deposit_usdc(102)]);
        let initial_1 = BalanceTree::from(vec![deposit_eth(20), deposit_usdc(201)]);

        let global_balance_tree: BalanceTreeByNetwork = {
            let base: BTreeMap<NetworkId, BalanceTree> =
                [(*NETWORK_A, initial_0), (*NETWORK_B, initial_1)].into();
            base.into()
        };

        State {
            global_exit_tree: [(*NETWORK_A, dummy_let.clone()), (*NETWORK_B, dummy_let.clone())]
                .into(),
            global_balance_tree,
        }
    }

    #[rstest]
    fn prove(initial_state: State, state_transition: Vec<Certificate>) {
        assert!(generate_leaf_proof(initial_state, state_transition).is_ok())
    }
}

#[test]
#[ignore = "not implemented yet"]
fn test_full_proof_mainnet_data() {
    // from data fetched from mainnet
}
