use leptos::prelude::*;
use leptos_router::components::Outlet;

use crate::components::sidebar::Sidebar;
use crate::components::toast::{Toast, ToastMsg};
use crate::i18n::provide_i18n;
use crate::theme::provide_theme;

#[component]
pub fn AppLayout() -> impl IntoView {
    let lang_signal = provide_i18n();
    let theme_signal = provide_theme();
    let toast = RwSignal::new(None::<ToastMsg>);

    provide_context(toast);

    view! {
        <div class="relative flex h-screen w-screen overflow-hidden">
            <Sidebar lang_signal=lang_signal theme_signal=theme_signal/>
            <main class="relative z-0 flex-1 overflow-y-auto bg-gray-50 dark:bg-slate-950">
                <div class="p-6">
                    <Outlet/>
                </div>
            </main>
            <Toast toast=toast.into()/>
        </div>
    }
}