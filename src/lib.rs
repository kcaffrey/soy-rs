#![allow(dead_code)]

pub mod error;

pub use self::tofu::Tofu;

mod ast;
mod parser;
mod tofu;
