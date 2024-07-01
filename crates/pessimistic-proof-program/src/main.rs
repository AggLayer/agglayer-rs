#![no_main]

use pessimistic_proof::{certificate::Certificate, generate_leaf_proof, LocalNetworkState};

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let initial_state = sp1_zkvm::io::read::<LocalNetworkState>();
    let certificate = sp1_zkvm::io::read::<Certificate>();

    let new_roots = generate_leaf_proof(initial_state, certificate).unwrap();

    sp1_zkvm::io::commit(&new_roots);
}
