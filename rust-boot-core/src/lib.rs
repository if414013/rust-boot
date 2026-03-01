//! Core traits, plugin system, shared types, and error handling.
//!
//! This crate provides the foundational abstractions for the rust-boot framework,
//! including the plugin system, application configuration, and error types.

pub mod config;
pub mod error;
pub mod plugin;
pub mod registry;
pub mod repository;
pub mod service;
