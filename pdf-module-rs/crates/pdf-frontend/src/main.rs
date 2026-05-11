#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::all)]

use leptos::mount::mount_to_body;

mod api;
mod app;
mod components;
mod i18n;
mod pages;
mod theme;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}