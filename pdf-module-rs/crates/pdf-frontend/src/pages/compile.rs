use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::toast::show_toast;
use crate::i18n::use_t;

#[component]
pub fn CompilePage() -> impl IntoView {
    let t = use_t();
    let status = RwSignal::new(None::<api::CompileStatusData>);
    let loading = RwSignal::new(false);

    loading.set(true);
    spawn_local(async move {
        match api::fetch_compile_status().await {
            Ok(data) => status.set(Some(data)),
            Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
        }
        loading.set(false);
    });

    view! {
        <div>
            <h1 class="text-xl font-semibold mb-6">{move || t("compile.title")}</h1>

            <div class="card mb-4">
                <Show
                    when=move || status.get().is_some()
                    fallback=move || view! {
                        <p class="text-sm text-gray-500 dark:text-slate-500">{move || t("compile.hint")}</p>
                    }
                >
                    <table class="w-full text-sm">
                        <tbody>
                            <tr class="border-t border-gray-200 dark:border-slate-800">
                                <td class="py-2 text-gray-500 dark:text-slate-500 font-mono text-xs w-40">{move || t("compile.running")}</td>
                                <td class="py-2">
                                    {move || {
                                        status.get().and_then(|s| s.running).map(|r| {
                                            if r { t("compile.yes") } else { t("compile.no") }
                                        }).unwrap_or(t("compile.no"))
                                    }}
                                </td>
                            </tr>
                            <tr class="border-t border-gray-200 dark:border-slate-800">
                                <td class="py-2 text-gray-500 dark:text-slate-500 font-mono text-xs">{move || t("compile.duration")}</td>
                                <td class="py-2">
                                    {move || {
                                        status.get().and_then(|s| s.last_duration_ms)
                                            .map(|d| format!("{} ms", d))
                                            .unwrap_or(t("compile.never"))
                                    }}
                                </td>
                            </tr>
                            <tr class="border-t border-gray-200 dark:border-slate-800">
                                <td class="py-2 text-gray-500 dark:text-slate-500 font-mono text-xs">{move || t("compile.outcome")}</td>
                                <td class="py-2">
                                    {move || {
                                        status.get().and_then(|s| s.last_outcome.clone())
                                            .unwrap_or(t("compile.never"))
                                    }}
                                </td>
                            </tr>
                            <tr class="border-t border-gray-200 dark:border-slate-800">
                                <td class="py-2 text-gray-500 dark:text-slate-500 font-mono text-xs">{move || t("compile.message")}</td>
                                <td class="py-2 text-gray-600 dark:text-slate-400">
                                    {move || {
                                        status.get().and_then(|s| s.message.clone())
                                            .unwrap_or_default()
                                    }}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </Show>
            </div>

            <div class="flex gap-3">
                <button class="btn" on:click=move |_| {
                    loading.set(true);
                    spawn_local(async move {
                        match api::fetch_compile_status().await {
                            Ok(data) => status.set(Some(data)),
                            Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
                        }
                        loading.set(false);
                    });
                }>
                    {move || if loading.get() { String::from("...") } else { t("compile.check") }}
                </button>
                <button class="btn btn-primary" on:click=move |_| {
                    spawn_local(async move {
                        match api::trigger_compile().await {
                            Ok(data) => {
                                show_toast(format!("{}: {}", t("toast.compile_done"), data.message.unwrap_or_default()), false);
                                status.set(None);
                            }
                            Err(e) => show_toast(format!("{}: {}", t("toast.failed"), e), true),
                        }
                    });
                }>
                    {move || t("compile.trigger")}
                </button>
            </div>
        </div>
    }
}