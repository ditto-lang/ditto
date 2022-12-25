use crate::{result::Warnings, supply::Supply};

#[derive(Debug, Default)]
pub struct State {
    pub supply: Supply,
    pub warnings: Warnings,
}
