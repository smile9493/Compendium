use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::toast::show_toast;
use crate::i18n::use_t;

#[component]
pub fn ConfigPage() -> impl IntoView {
    let t = use_t();
    let config = RwSignal::new(None::<std::collections::HashMap<String, String>>);
    let key_input = RwSignal::new(String::new());
    let val_input = RwSignal::new(String::new());

    let load = {
        let t = t.clone();
        move |_| {
            spawn_local(async move {
                match api::fetch_config().await {
                    Ok(data) => config.set(Some(data.config.unwrap_or_default())),
                    Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
                }
            });
        }
    };

    let set_cfg = {
        let t = t.clone();
        let key_input = key_input;
        let val_input = val_input;
        move |_| {
            let key = key_input.get();
            let val = val_input.get();
            if key.is_empty() {
                show_toast(t("toast.key_required"), true);
                return;
            }
            spawn_local(async move {
                match api::set_config(&key, &val).await {
                    Ok(()) => {
                        show_toast(format!("{}: {}", t("toast.config_set"), key), false);
                        key_input.set(String::new());
                        val_input.set(String::new());
                        load(());
                    }
                    Err(e) => show_toast(format!("{}: {}", t("toast.failed"), e), true),
                }
            });
        }
    };

    let remove_cfg = {
        let t = t.clone();
        move |key: String| {
            spawn_local(async move {
                match api::delete_config(&key).await {
                    Ok(()) => {
                        show_toast(format!("{}: {}", t("toast.config_removed"), key), false);
                        load(());
                    }
                    Err(e) => show_toast(format!("{}: {}", t("toast.failed"), e), true),
                }
            });
        }
    };

    load(());

    view! {
        <div>
            <h1 class="text-xl font-semibold mb-6">{move || t("config.title")}</h1>

            <div class="card mb-6">
                <table class="w-full text-sm">
                    <thead>
                        <tr>
                            <th class="text-left pb-2 text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 font-medium">{move || t("config.key")}</th>
                            <th class="text-left pb-2 text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 font-medium">{move || t("config.value")}</th>
                            <th class="text-right pb-2 text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 font-medium">{move || t("config.actions")}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            match config.get() {
                                Some(ref entries) if !entries.is_empty() => {
                                    let mut sorted: Vec<_> = entries.iter().collect();
                                    sorted.sort_by(|a, b| a.0.cmp(b.0));
                                    sorted.iter().map(|(k, v)| {
                                        let kc = String::clone(k);
                                        view! {
                                            <tr class="border-t border-gray-200 dark:border-slate-800">
                                                <td class="py-2 font-mono text-blue-600 dark:text-blue-400">{String::clone(k)}</td>
                                                <td class="py-2 text-gray-600 dark:text-slate-400 truncate max-w-[300px]">{String::clone(v)}</td>
                                                <td class="py-2 text-right">
                                                    <button
                                                        class="text-xs px-2 py-1 border border-gray-300 dark:border-slate-700 rounded hover:border-red-500 hover:text-red-400 transition-colors"
                                                        on:click={
                                                            let kc = kc.clone();
                                                            move |_| remove_cfg(kc.clone())
                                                        }
                                                    >
                                                        {t("config.remove")}
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view().into_any()
                                }
                                _ => {
                                    view! { <tr><td colspan="3" class="py-4 text-center text-gray-400 dark:text-slate-600">{t("config.loading")}</td></tr> }.into_any()
                                }
                            }
                        }}
                    </tbody>
                </table>
            </div>

            <div class="card">
                <div class="flex gap-3">
                    <input
                        type="text"
                        class="flex-1 px-3 py-2 text-sm bg-white dark:bg-slate-950 border border-gray-300 dark:border-slate-700 rounded-lg text-gray-900 dark:text-slate-200
                               placeholder-gray-400 dark:placeholder-slate-600 focus:outline-none focus:border-blue-500 transition-colors"
                        placeholder=move || t("config.placeholder_key")
                        prop:value=key_input
                        on:input=move |ev| key_input.set(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        class="flex-1 px-3 py-2 text-sm bg-white dark:bg-slate-950 border border-gray-300 dark:border-slate-700 rounded-lg text-gray-900 dark:text-slate-200
                               placeholder-gray-400 dark:placeholder-slate-600 focus:outline-none focus:border-blue-500 transition-colors"
                        placeholder=move || t("config.placeholder_value")
                        prop:value=val_input
                        on:input=move |ev| val_input.set(event_target_value(&ev))
                    />
                    <button class="btn btn-primary" on:click=set_cfg>
                        {move || t("config.set")}
                    </button>
                </div>
            </div>
        </div>
    }
}