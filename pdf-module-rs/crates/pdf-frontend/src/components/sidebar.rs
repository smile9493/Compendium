use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use crate::i18n::{use_t, LangSignal};
use crate::theme::ThemeSignal;

#[component]
pub fn Sidebar(lang_signal: LangSignal, theme_signal: ThemeSignal) -> impl IntoView {
    let t = use_t();
    let collapsed = RwSignal::new(false);
    let navigate = use_navigate();

    let nav_items = [
        ("/", "M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z", "nav.dashboard"),
        ("/wiki", "M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25", "nav.wiki"),
        ("/config", "M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.086.22-.128.332-.183.582-.495.644-.869l.214-1.28Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z", "nav.config"),
        ("/compile", "M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 0 0-2.455 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z", "nav.compile"),
    ];

    view! {
        <aside class="relative z-10 flex flex-col h-full bg-white dark:bg-slate-900 border-r border-gray-200 dark:border-slate-800 shrink-0 transition-all duration-200"
            style=move || if collapsed.get() { "width: 4rem" } else { "width: 16rem" }
        >
            <div class="flex items-center justify-between h-12 px-4 border-b border-gray-200 dark:border-slate-800 shrink-0">
                <span class=move || if collapsed.get() { "hidden" } else { "font-mono text-xs text-gray-600 dark:text-slate-400 truncate" }>
                    "rsut-pdf-mcp"
                </span>
                <button
                    on:click=move |_| collapsed.update(|c| *c = !*c)
                    class="p-1.5 text-gray-500 dark:text-slate-500 hover:text-gray-700 dark:hover:text-slate-300 transition-colors rounded hover:bg-gray-100 dark:hover:bg-slate-800"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=move || {
                            if collapsed.get() { "M13 5l7 7-7 7" } else { "M11 19l-7-7 7-7" }
                        }/>
                    </svg>
                </button>
            </div>

            <nav class="flex-1 py-2 overflow-y-auto">
                {nav_items.into_iter().map(|(href, path_d, key)| {
                    let navigate = navigate.clone();
                    let href_owned = href.to_string();
                    view! {
                        <a
                            href=href
                            class="flex items-center gap-3 px-4 py-2.5 text-sm text-gray-600 dark:text-slate-400 hover:text-gray-900 dark:hover:text-slate-200 hover:bg-gray-100 dark:hover:bg-slate-800/50 transition-colors cursor-pointer"
                            on:click=move |ev| {
                                ev.prevent_default();
                                navigate(&href_owned, NavigateOptions::default());
                            }
                        >
                            <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=path_d/>
                            </svg>
                            <span class=move || if collapsed.get() { "sr-only" } else { "" }>
                                {t(key)}
                            </span>
                        </a>
                    }
                }).collect_view()}
            </nav>

            <div class="p-3 border-t border-gray-200 dark:border-slate-800 shrink-0 space-y-2">
                <button
                    on:click=move |_| theme_signal.toggle()
                    class=move || {
                        let base = "w-full py-1.5 text-xs rounded transition-colors border ";
                        if collapsed.get() {
                            format!("{}px-2 text-gray-500 dark:text-slate-500 hover:text-gray-700 dark:hover:text-slate-300 border-gray-300 dark:border-slate-700 hover:bg-gray-100 dark:hover:bg-slate-800", base)
                        } else {
                            format!("{}px-3 text-gray-600 dark:text-slate-400 hover:text-gray-900 dark:hover:text-slate-200 border-gray-300 dark:border-slate-700 hover:bg-gray-100 dark:hover:bg-slate-800", base)
                        }
                    }
                >
                    {move || match theme_signal.theme.get() {
                        crate::theme::Theme::Light => "🌙",
                        crate::theme::Theme::Dark => "☀️",
                    }}
                </button>

                <button
                    on:click=move |_| {
                        let new = match lang_signal.lang.get() {
                            crate::i18n::Lang::Zh => crate::i18n::Lang::En,
                            crate::i18n::Lang::En => crate::i18n::Lang::Zh,
                        };
                        lang_signal.lang.set(new);
                    }
                    class=move || {
                        let base = "w-full py-1.5 text-xs rounded transition-colors border ";
                        if collapsed.get() {
                            format!("{}px-2 text-gray-500 dark:text-slate-500 hover:text-gray-700 dark:hover:text-slate-300 border-gray-300 dark:border-slate-700", base)
                        } else {
                            format!("{}px-3 text-gray-600 dark:text-slate-400 hover:text-gray-900 dark:hover:text-slate-200 border-gray-300 dark:border-slate-700", base)
                        }
                    }
                >
                    {move || match lang_signal.lang.get() {
                        crate::i18n::Lang::Zh => "EN",
                        crate::i18n::Lang::En => "中文",
                    }}
                </button>
            </div>
        </aside>
    }
}