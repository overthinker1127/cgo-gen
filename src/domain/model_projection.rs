use crate::domain::kind::IrTypeKind;

#[derive(Debug, Clone)]
pub struct ModelProjection {
    pub cpp_type: String,
    pub go_name: String,
    pub handle_name: String,
    pub constructor_symbol: String,
    pub destructor_symbol: String,
    pub fields: Vec<ModelProjectionField>,
}

#[derive(Debug, Clone)]
pub struct ModelProjectionField {
    pub go_name: String,
    pub go_type: String,
    pub getter_symbol: String,
    pub setter_symbol: String,
    pub return_kind: IrTypeKind,
}
