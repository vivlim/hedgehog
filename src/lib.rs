#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod authenticate;
pub mod channels;
pub mod service;
pub use app::TemplateApp;
