//! DTO code generation for CreateDTO, UpdateDTO, and ResponseDTO.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::ir::{CrudModelIr, ValidationRule};

pub fn generate_dtos(ir: &CrudModelIr) -> TokenStream {
    let create_dto = generate_create_dto(ir);
    let update_dto = generate_update_dto(ir);
    let response_dto = generate_response_dto(ir);

    quote! {
        #create_dto
        #update_dto
        #response_dto
    }
}

fn generate_create_dto(ir: &CrudModelIr) -> TokenStream {
    let model_name = &ir.name;
    let dto_name = format_ident!("Create{}Dto", model_name);

    let fields: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.is_primary_key && !f.skip_dto)
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            let validation_attrs = generate_validation_attrs(&f.validations);

            if validation_attrs.is_empty() {
                quote! { pub #name: #ty, }
            } else {
                quote! {
                    #(#validation_attrs)*
                    pub #name: #ty,
                }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct #dto_name {
            #(#fields)*
        }
    }
}

fn generate_update_dto(ir: &CrudModelIr) -> TokenStream {
    let model_name = &ir.name;
    let dto_name = format_ident!("Update{}Dto", model_name);

    let fields: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.is_primary_key && !f.skip_dto)
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            let validation_attrs = generate_validation_attrs(&f.validations);

            let optional_ty = make_optional(ty);

            if validation_attrs.is_empty() {
                quote! { pub #name: #optional_ty, }
            } else {
                quote! {
                    #(#validation_attrs)*
                    pub #name: #optional_ty,
                }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Default, serde::Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct #dto_name {
            #(#fields)*
        }
    }
}

fn generate_response_dto(ir: &CrudModelIr) -> TokenStream {
    let model_name = &ir.name;
    let dto_name = format_ident!("{}Response", model_name);

    let mut fields: Vec<TokenStream> = ir
        .fields
        .iter()
        .filter(|f| !f.skip_dto)
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            quote! { pub #name: #ty, }
        })
        .collect();

    if ir.timestamps {
        fields.push(quote! { pub created_at: chrono::DateTime<chrono::Utc>, });
        fields.push(quote! { pub updated_at: chrono::DateTime<chrono::Utc>, });
    }

    quote! {
        #[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct #dto_name {
            #(#fields)*
        }
    }
}

fn make_optional(ty: &syn::Type) -> TokenStream {
    if is_option_type(ty) {
        quote! { #ty }
    } else {
        quote! { Option<#ty> }
    }
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn generate_validation_attrs(validations: &[ValidationRule]) -> Vec<TokenStream> {
    validations
        .iter()
        .filter_map(|v| match v {
            ValidationRule::Email => Some(quote! { #[validate(email)] }),
            ValidationRule::Url => Some(quote! { #[validate(url)] }),
            ValidationRule::MinLength(len) => Some(quote! { #[validate(length(min = #len))] }),
            ValidationRule::MaxLength(len) => Some(quote! { #[validate(length(max = #len))] }),
            ValidationRule::Pattern(pattern) => Some(quote! { #[validate(regex = #pattern)] }),
            ValidationRule::Custom(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::FieldIr;
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
                    validations: vec![ValidationRule::Email],
                },
                FieldIr {
                    name: format_ident!("name"),
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
    fn test_generate_create_dto() {
        let ir = create_test_ir();
        let tokens = generate_create_dto(&ir);
        let code = tokens.to_string();

        assert!(code.contains("CreateUserDto"));
        assert!(code.contains("Deserialize"));
        assert!(code.contains("ToSchema"));
        assert!(code.contains("camelCase"));
        assert!(code.contains("email"));
        assert!(code.contains("name"));
        assert!(!code.contains("pub id"));
    }

    #[test]
    fn test_generate_update_dto() {
        let ir = create_test_ir();
        let tokens = generate_update_dto(&ir);
        let code = tokens.to_string();

        assert!(code.contains("UpdateUserDto"));
        assert!(code.contains("Option"));
        assert!(code.contains("Default"));
    }

    #[test]
    fn test_generate_response_dto() {
        let ir = create_test_ir();
        let tokens = generate_response_dto(&ir);
        let code = tokens.to_string();

        assert!(code.contains("UserResponse"));
        assert!(code.contains("Serialize"));
        assert!(code.contains("pub id"));
        assert!(code.contains("pub email"));
    }

    #[test]
    fn test_response_dto_with_timestamps() {
        let mut ir = create_test_ir();
        ir.timestamps = true;
        let tokens = generate_response_dto(&ir);
        let code = tokens.to_string();

        assert!(code.contains("created_at"));
        assert!(code.contains("updated_at"));
    }

    #[test]
    fn test_create_dto_excludes_primary_key() {
        let ir = create_test_ir();
        let tokens = generate_create_dto(&ir);
        let code = tokens.to_string();

        assert!(!code.contains("pub id :"));
    }

    #[test]
    fn test_skip_dto_field() {
        let mut ir = create_test_ir();
        ir.fields.push(FieldIr {
            name: format_ident!("password_hash"),
            ty: parse_quote!(String),
            column_name: None,
            is_primary_key: false,
            is_nullable: false,
            skip_dto: true,
            default_value: None,
            validations: vec![],
        });

        let create_code = generate_create_dto(&ir).to_string();
        let response_code = generate_response_dto(&ir).to_string();

        assert!(!create_code.contains("password_hash"));
        assert!(!response_code.contains("password_hash"));
    }

    #[test]
    fn test_validation_attrs_email() {
        let validations = vec![ValidationRule::Email];
        let attrs = generate_validation_attrs(&validations);

        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].to_string().contains("email"));
    }

    #[test]
    fn test_validation_attrs_length() {
        let validations = vec![ValidationRule::MinLength(5), ValidationRule::MaxLength(100)];
        let attrs = generate_validation_attrs(&validations);

        assert_eq!(attrs.len(), 2);
        let code = attrs.iter().map(|a| a.to_string()).collect::<String>();
        assert!(code.contains("min = 5"));
        assert!(code.contains("max = 100"));
    }

    #[test]
    fn test_is_option_type() {
        let option_ty: syn::Type = parse_quote!(Option<String>);
        let string_ty: syn::Type = parse_quote!(String);

        assert!(is_option_type(&option_ty));
        assert!(!is_option_type(&string_ty));
    }

    #[test]
    fn test_make_optional_already_option() {
        let ty: syn::Type = parse_quote!(Option<String>);
        let result = make_optional(&ty);

        assert!(!result.to_string().contains("Option < Option"));
    }

    #[test]
    fn test_generate_dtos_all() {
        let ir = create_test_ir();
        let tokens = generate_dtos(&ir);
        let code = tokens.to_string();

        assert!(code.contains("CreateUserDto"));
        assert!(code.contains("UpdateUserDto"));
        assert!(code.contains("UserResponse"));
    }
}
