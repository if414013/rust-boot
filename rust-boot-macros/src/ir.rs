//! Intermediate representation for CrudModel macro parsing.

use syn::Ident;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationRule {
    Email,
    Url,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Custom(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct FieldIr {
    pub name: Ident,
    pub ty: syn::Type,
    pub column_name: Option<String>,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    pub skip_dto: bool,
    pub default_value: Option<String>,
    pub validations: Vec<ValidationRule>,
}

#[derive(Debug, Clone)]
pub struct CrudModelIr {
    pub name: Ident,
    pub table_name: String,
    pub soft_delete: bool,
    pub timestamps: bool,
    pub fields: Vec<FieldIr>,
}

#[allow(dead_code)]
impl CrudModelIr {
    pub fn primary_key_field(&self) -> Option<&FieldIr> {
        self.fields.iter().find(|f| f.is_primary_key)
    }

    pub fn dto_fields(&self) -> impl Iterator<Item = &FieldIr> {
        self.fields.iter().filter(|f| !f.skip_dto)
    }

    pub fn required_fields(&self) -> impl Iterator<Item = &FieldIr> {
        self.fields
            .iter()
            .filter(|f| !f.is_nullable && !f.is_primary_key)
    }
}
