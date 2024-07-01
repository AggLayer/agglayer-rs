use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProverConfig {
    SP1Network {},
    SP1Local {},
    SP1Mock {},
}
