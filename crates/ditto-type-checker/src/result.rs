use crate::{error::Error, outputs::Outputs};

pub type Result<T> = std::result::Result<(T, Outputs), Error>;
