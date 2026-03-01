//! Procedural macros for deriving and generating code for the rust-boot framework.
//!
//! This crate provides the `#[derive(CrudModel)]` macro for auto-generating
//! CRUD-related code including entities, DTOs, and service implementations.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod gen;
mod ir;
mod parse;

/// Derives CRUD model implementations for a struct.
///
/// This macro generates:
/// - SeaORM entity with columns and relations
/// - CreateDTO, UpdateDTO, and ResponseDTO structs
/// - OpenAPI schema annotations via utoipa
///
/// # Attributes
///
/// ## Struct-level attributes
///
/// - `#[crud_model(table_name = "users")]` - Database table name
/// - `#[crud_model(soft_delete)]` - Enable soft delete with `deleted_at` column
/// - `#[crud_model(timestamps)]` - Auto-generate `created_at` and `updated_at`
///
/// ## Field-level attributes
///
/// - `#[crud_field(primary_key)]` - Mark as primary key
/// - `#[crud_field(column_name = "user_name")]` - Custom column name
/// - `#[crud_field(nullable)]` - Allow NULL values
/// - `#[crud_field(skip_dto)]` - Exclude from DTOs
/// - `#[crud_field(validation = "email")]` - Add validation rule
///
/// # Example
///
/// ```ignore
/// use rust_boot_macros::CrudModel;
///
/// #[derive(CrudModel)]
/// #[crud_model(table_name = "users", soft_delete, timestamps)]
/// pub struct User {
///     #[crud_field(primary_key)]
///     pub id: i64,
///     
///     #[crud_field(validation = "email")]
///     pub email: String,
///     
///     #[crud_field(nullable)]
///     pub bio: Option<String>,
/// }
/// ```
#[proc_macro_derive(CrudModel, attributes(crud_model, crud_field))]
pub fn derive_crud_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ir = match parse::parse_crud_model(&input) {
        Ok(ir) => ir,
        Err(err) => return err.to_compile_error().into(),
    };

    let expanded = generate_code(&ir);
    TokenStream::from(expanded)
}

fn generate_code(ir: &ir::CrudModelIr) -> proc_macro2::TokenStream {
    let entity = gen::entity::generate_entity(ir);
    let dtos = gen::dto::generate_dtos(ir);

    quote::quote! {
        #entity
        #dtos
    }
}
