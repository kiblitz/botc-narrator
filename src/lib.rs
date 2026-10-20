//! A Blood on the Clocktower storyteller engine.
//!
//! Characters describe their abilities against [`ctx::Read`] / [`ctx::Write`];
//! how players are talked to ([`table::Io`]) and how the storyteller makes
//! discretionary choices ([`storyteller::Strategy`]) are pluggable.

pub mod character;
pub mod characters;
pub mod ctx;
pub mod event;
pub mod game;
pub mod info;
pub mod narrator;
pub mod player;
pub mod script;
pub mod storyteller;
pub mod table;
pub mod trouble_brewing;
