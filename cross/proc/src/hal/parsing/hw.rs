use super::*;

#[derive(Debug, Parse)]
pub struct Hw {
    pub(super) _token: tokens::hw,
    pub(super) _colon: Token![:],
    pub(super) fields: syn::FieldsNamed,
}
