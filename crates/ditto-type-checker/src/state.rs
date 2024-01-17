use crate::{substitution::Substitution, supply::Supply};

pub struct State {
    pub supply: Supply,
    pub substitution: Substitution,
}
