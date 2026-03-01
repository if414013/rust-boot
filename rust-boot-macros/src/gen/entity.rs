//! SeaORM entity code generation from CrudModelIr.

use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::{CrudModelIr, FieldIr};

pub fn generate_entity(ir: &CrudModelIr) -> TokenStream {
    let model_name = &ir.name;
    let table_name = &ir.table_name;
    let soft_delete = ir.soft_delete;
    let timestamps = ir.timestamps;

    let field_attrs = generate_field_attributes(&ir.fields, timestamps, soft_delete);

    quote! {
        impl #model_name {
            pub const fn table_name() -> &'static str {
                #table_name
            }

            pub const fn soft_delete_enabled() -> bool {
                #soft_delete
            }

            pub const fn timestamps_enabled() -> bool {
                #timestamps
            }
        }

        pub mod entity {
            use sea_orm::entity::prelude::*;

            #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
            #[sea_orm(table_name = #table_name)]
            pub struct Model {
                #(#field_attrs)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            impl ActiveModelBehavior for ActiveModel {}
        }
    }
}

fn generate_field_attributes(
    fields: &[FieldIr],
    timestamps: bool,
    soft_delete: bool,
) -> Vec<TokenStream> {
    let mut result: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let name = &field.name;
            let ty = &field.ty;
            let column_name = field
                .column_name
                .clone()
                .unwrap_or_else(|| to_snake_case(&name.to_string()));

            let mut attrs = Vec::new();

            if field.is_primary_key {
                attrs.push(quote! { #[sea_orm(primary_key)] });
            }

            let field_snake = to_snake_case(&name.to_string());
            if column_name != field_snake {
                attrs.push(quote! { #[sea_orm(column_name = #column_name)] });
            }

            if field.is_nullable {
                attrs.push(quote! { #[sea_orm(nullable)] });
            }

            if attrs.is_empty() {
                quote! {
                    pub #name: #ty,
                }
            } else {
                quote! {
                    #(#attrs)*
                    pub #name: #ty,
                }
            }
        })
        .collect();

    if timestamps {
        result.push(quote! {
            pub created_at: DateTimeUtc,
        });
        result.push(quote! {
            pub updated_at: DateTimeUtc,
        });
    }

    if soft_delete {
        result.push(quote! {
            #[sea_orm(nullable)]
            pub deleted_at: Option<DateTimeUtc>,
        });
    }

    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

#[allow(dead_code)]
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::format_ident;
    use syn::parse_quote;

    fn create_test_ir() -> CrudModelIr {
        CrudModelIr {
            name: format_ident!("User"),
            table_name: "users".to_string(),
            soft_delete: false,
            timestamps: false,
            fields: vec![
                FieldIr {
                    name: format_ident!("id"),
                    ty: parse_quote!(i64),
                    column_name: None,
                    is_primary_key: true,
                    is_nullable: false,
                    skip_dto: false,
                    default_value: None,
                    validations: vec![],
                },
                FieldIr {
                    name: format_ident!("email"),
                    ty: parse_quote!(String),
                    column_name: None,
                    is_primary_key: false,
                    is_nullable: false,
                    skip_dto: false,
                    default_value: None,
                    validations: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_generate_entity_basic() {
        let ir = create_test_ir();
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("table_name"));
        assert!(code.contains("users"));
        assert!(code.contains("DeriveEntityModel"));
        assert!(code.contains("pub struct Model"));
        assert!(code.contains("pub enum Relation"));
    }

    #[test]
    fn test_generate_entity_with_primary_key() {
        let ir = create_test_ir();
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("primary_key"));
    }

    #[test]
    fn test_generate_entity_with_timestamps() {
        let mut ir = create_test_ir();
        ir.timestamps = true;
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("created_at"));
        assert!(code.contains("updated_at"));
        assert!(code.contains("DateTimeUtc"));
    }

    #[test]
    fn test_generate_entity_with_soft_delete() {
        let mut ir = create_test_ir();
        ir.soft_delete = true;
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("deleted_at"));
        assert!(code.contains("Option < DateTimeUtc >") || code.contains("Option<DateTimeUtc>"));
    }

    #[test]
    fn test_generate_entity_with_nullable_field() {
        let mut ir = create_test_ir();
        ir.fields.push(FieldIr {
            name: format_ident!("bio"),
            ty: parse_quote!(Option<String>),
            column_name: None,
            is_primary_key: false,
            is_nullable: true,
            skip_dto: false,
            default_value: None,
            validations: vec![],
        });
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("nullable"));
        assert!(code.contains("bio"));
    }

    #[test]
    fn test_generate_entity_with_custom_column_name() {
        let mut ir = create_test_ir();
        ir.fields[1].column_name = Some("user_email".to_string());
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("column_name"));
        assert!(code.contains("user_email"));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("userId"), "user_id");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("user_id"), "UserId");
    }

    #[test]
    fn test_soft_delete_enabled_const() {
        let mut ir = create_test_ir();
        ir.soft_delete = true;
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("soft_delete_enabled"));
        assert!(code.contains("true"));
    }

    #[test]
    fn test_timestamps_enabled_const() {
        let mut ir = create_test_ir();
        ir.timestamps = true;
        let tokens = generate_entity(&ir);
        let code = tokens.to_string();

        assert!(code.contains("timestamps_enabled"));
        assert!(code.contains("true"));
    }
}
