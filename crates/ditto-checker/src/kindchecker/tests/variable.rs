use super::macros::*;
use crate::TypeError::*;

#[test]
fn it_errors_as_expected() {
    assert_type_error!("a", UnknownTypeVariable { .. });
}
