//! Parsing logic for `CrudModel` attributes.

use darling::{FromDeriveInput, FromField};
use syn::{DeriveInput, Error, Ident};

use crate::ir::{CrudModelIr, FieldIr, ValidationRule};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(crud_model), supports(struct_named))]
struct CrudModelAttrs {
    ident: Ident,
    data: darling::ast::Data<(), CrudFieldAttrs>,

    #[darling(default)]
    table_name: Option<String>,

    #[darling(default)]
    soft_delete: bool,

    #[darling(default)]
    timestamps: bool,
}

#[derive(Debug, FromField)]
#[darling(attributes(crud_field))]
struct CrudFieldAttrs {
    ident: Option<Ident>,
    ty: syn::Type,

    #[darling(default)]
    primary_key: bool,

    #[darling(default)]
    column_name: Option<String>,

    #[darling(default)]
    nullable: bool,

    #[darling(default)]
    skip_dto: bool,

    #[darling(default)]
    default: Option<String>,

    #[darling(default)]
    validation: Option<String>,
}

pub fn parse_crud_model(input: &DeriveInput) -> Result<CrudModelIr, Error> {
    let attrs = CrudModelAttrs::from_derive_input(input)?;

    let table_name = attrs
        .table_name
        .unwrap_or_else(|| to_snake_case(&attrs.ident.to_string()));

    let fields = match attrs.data {
        darling::ast::Data::Struct(fields) => fields
            .fields
            .into_iter()
            .map(parse_field)
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(Error::new_spanned(
                input,
                "CrudModel can only be derived for structs with named fields",
            ))
        }
    };

    Ok(CrudModelIr {
        name: attrs.ident,
        table_name,
        soft_delete: attrs.soft_delete,
        timestamps: attrs.timestamps,
        fields,
    })
}

fn parse_field(attrs: CrudFieldAttrs) -> Result<FieldIr, Error> {
    let name = attrs.ident.ok_or_else(|| {
        Error::new(
            proc_macro2::Span::call_site(),
            "CrudModel fields must be named",
        )
    })?;

    let validations = parse_validation(&attrs.validation)?;

    Ok(FieldIr {
        name,
        ty: attrs.ty,
        column_name: attrs.column_name,
        is_primary_key: attrs.primary_key,
        is_nullable: attrs.nullable,
        skip_dto: attrs.skip_dto,
        default_value: attrs.default,
        validations,
    })
}

fn parse_validation(validation: &Option<String>) -> Result<Vec<ValidationRule>, Error> {
    let Some(val) = validation else {
        return Ok(Vec::new());
    };

    let mut rules = Vec::new();
    for rule in val.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let parsed = match rule {
            "email" => ValidationRule::Email,
            "url" => ValidationRule::Url,
            s if s.starts_with("min_length:") => {
                let len: usize = s
                    .strip_prefix("min_length:")
                    .unwrap()
                    .parse()
                    .map_err(|_| {
                        Error::new(proc_macro2::Span::call_site(), "Invalid min_length value")
                    })?;
                ValidationRule::MinLength(len)
            }
            s if s.starts_with("max_length:") => {
                let len: usize = s
                    .strip_prefix("max_length:")
                    .unwrap()
                    .parse()
                    .map_err(|_| {
                        Error::new(proc_macro2::Span::call_site(), "Invalid max_length value")
                    })?;
                ValidationRule::MaxLength(len)
            }
            s if s.starts_with("pattern:") => {
                let pattern = s.strip_prefix("pattern:").unwrap().to_string();
                ValidationRule::Pattern(pattern)
            }
            s => ValidationRule::Custom(s.to_string()),
        };
        rules.push(parsed);
    }

    Ok(rules)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("HTTPRequest"), "h_t_t_p_request");
        assert_eq!(to_snake_case("user"), "user");
    }

    #[test]
    fn test_parse_validation_empty() {
        let result = parse_validation(&None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_validation_email() {
        let result = parse_validation(&Some("email".to_string())).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ValidationRule::Email);
    }

    #[test]
    fn test_parse_validation_multiple() {
        let result =
            parse_validation(&Some("email, min_length:5, max_length:100".to_string())).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ValidationRule::Email);
        assert_eq!(result[1], ValidationRule::MinLength(5));
        assert_eq!(result[2], ValidationRule::MaxLength(100));
    }

    #[test]
    fn test_parse_validation_pattern() {
        let result = parse_validation(&Some("pattern:^[a-z]+$".to_string())).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], ValidationRule::Pattern("^[a-z]+$".to_string()));
    }

    #[test]
    fn test_parse_validation_custom() {
        let result = parse_validation(&Some("my_custom_rule".to_string())).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            ValidationRule::Custom("my_custom_rule".to_string())
        );
    }
}
