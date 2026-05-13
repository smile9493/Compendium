use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::tree_view::TreeView;
use crate::components::toast::show_toast;
use crate::i18n::use_t;

#[component]
pub fn WikiPage() -> impl IntoView {
    let t = use_t();
    let tree = RwSignal::new(None::<api::TreeNode>);
    let entry = RwSignal::new(None::<api::WikiEntry>);
    let active_path = RwSignal::new(None::<String>);
    let search_results = RwSignal::new(None::<Vec<api::SearchHit>>);
    let stats_text = RwSignal::new(String::new());
    let concept_map = RwSignal::new(None::<String>);
    let show_concept = RwSignal::new(false);

    spawn_local(async move {
        if let Ok(data) = api::fetch_wiki_tree().await {
            tree.set(data.tree);
        }
    });

    spawn_local(async move {
        if let Ok(data) = api::fetch_wiki_stats().await {
            let entries = data.total_entries.unwrap_or(0);
            let domains = data.domains.as_ref().map(|d| d.len()).unwrap_or(0);
            stats_text.set(format!(
                "{} {} · {} {}",
                entries,
                t("wiki.entries"),
                domains,
                t("wiki.domains")
            ));
        }
    });

    let select_entry = move |path: String| {
        active_path.set(Some(path.clone()));
        entry.set(None);
        search_results.set(None);
        show_concept.set(false);
        spawn_local(async move {
            match api::fetch_wiki_entry(&path).await {
                Ok(data) => {
                    if let Some(e) = data.entry {
                        entry.set(Some(e));
                    } else {
                        show_toast(data.error.unwrap_or_else(|| "Unknown error".into()), true);
                    }
                }
                Err(e) => show_toast(e, true),
            }
        });
    };

    view! {
        <div class="flex h-[calc(100vh-3rem)] -m-6">
            <aside class="w-72 bg-gray-100 dark:bg-slate-900/50 border-r border-gray-200 dark:border-slate-800 flex flex-col shrink-0">
                <div class="p-3 border-b border-gray-200 dark:border-slate-800">
                    <div class="relative">
                        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400 dark:text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
                        </svg>
                        <input
                            type="text"
                            class="w-full pl-9 pr-3 py-2 text-xs bg-white dark:bg-slate-950 border border-gray-300 dark:border-slate-700 rounded-lg text-gray-900 dark:text-slate-200
                                   placeholder-gray-400 dark:placeholder-slate-600 focus:outline-none focus:border-teal-500 transition-colors"
                            placeholder=move || t("wiki.search")
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                if val.trim().is_empty() {
                                    search_results.set(None);
                                    return;
                                }
                                spawn_local(async move {
                                    match api::search_wiki(&val).await {
                                        Ok(data) => search_results.set(data.results),
                                        Err(e) => show_toast(e, true),
                                    }
                                });
                            }
                        />
                    </div>
                </div>

                <Show
                    when=move || search_results.get().is_none()
                    fallback=move || view! {
                        <div class="flex-1 overflow-y-auto">
                            {move || {
                                search_results.get().map(|results| {
                                    view! {
                                        <div>
                                            {results.iter().map(|hit| {
                                                let path = hit.path.clone().unwrap_or_default();
                                                let title = hit.title.clone().unwrap_or_default();
                                                let score = hit.score.unwrap_or(0.0);
                                                view! {
                                                    <div
                                                        class="px-4 py-3 border-b border-gray-200 dark:border-slate-800 cursor-pointer hover:bg-teal-50 dark:hover:bg-teal-950/20 transition-colors"
                                                        on:click={
                                                            let p = path.clone();
                                                            move |_| {
                                                                search_results.set(None);
                                                                select_entry(p.clone());
                                                            }
                                                        }
                                                    >
                                                        <div class="text-sm font-medium text-gray-700 dark:text-slate-300">{title}</div>
                                                        <div class="text-xs text-gray-400 dark:text-slate-600 font-mono mt-1">
                                                            {format!("{} · score: {:.2}", path, score)}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }
                                })
                            }}
                        </div>
                    }
                >
                    <div class="flex-1 overflow-y-auto">
                        <TreeView
                            node=Signal::derive(move || tree.get())
                            on_select=Callback::new(move |p| select_entry(p))
                            active_path=Signal::derive(move || active_path.get())
                        />
                    </div>
                </Show>

                <div class="p-3 border-t border-gray-200 dark:border-slate-800 text-xs text-gray-400 dark:text-slate-600 font-mono">
                    {move || stats_text.get()}
                </div>
            </aside>

            <div class="flex-1 overflow-y-auto">
                <Show
                    when=move || entry.get().is_some()
                    fallback=move || view! {
                        <div class="flex flex-col items-center justify-center h-full text-gray-500 dark:text-slate-500">
                            <svg class="w-16 h-16 mb-4 opacity-20" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"/>
                            </svg>
                            <h3 class="text-sm font-medium mb-1">{move || t("wiki.empty")}</h3>
                            <p class="text-xs">{move || t("wiki.empty_desc")}</p>
                        </div>
                    }
                >
                    {move || {
                        entry.get().map(|e| {
                            view! {
                                <div class="max-w-4xl mx-auto px-8 py-8">
                                    <div class="flex flex-wrap gap-2 mb-6">
                                        {e.domain.clone().map(|d| view! { <span class="px-2 py-0.5 rounded-full text-xs font-mono bg-blue-950/30 text-blue-400">{d}</span> })}
                                        {e.level.clone().map(|l| view! { <span class="px-2 py-0.5 rounded-full text-xs font-mono bg-green-950/30 text-green-400">{l}</span> })}
                                        {e.status.clone().map(|s| view! { <span class="px-2 py-0.5 rounded-full text-xs font-mono bg-yellow-950/30 text-yellow-400">{s}</span> })}
                                        {e.quality_score.map(|qs| view! { <span class="px-2 py-0.5 rounded-full text-xs font-mono bg-teal-950/30 text-teal-400">{format!("{}: {:.0}%", t("wiki.quality"), qs * 100.0)}</span> })}
                                    </div>

                                    <h1 class="text-2xl font-bold mb-8 text-gray-900 dark:text-slate-100">{e.title.clone().unwrap_or_default()}</h1>

                                    {e.body_html.clone().map(|body| {
                                        view! {
                                            <div class="prose prose-sm max-w-3xl
                                                prose-headings:text-gray-900 dark:prose-headings:text-slate-100 prose-h1:text-xl prose-h2:text-lg
                                                prose-p:text-gray-700 dark:prose-p:text-slate-300 prose-p:leading-relaxed
                                                prose-code:bg-gray-100 dark:prose-code:bg-slate-800 prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:font-mono prose-code:text-sm
                                                prose-pre:bg-gray-50 dark:prose-pre:bg-slate-900 prose-pre:border prose-pre:border-gray-200 dark:prose-pre:border-slate-800
                                                prose-a:text-teal-600 dark:prose-a:text-teal-400 prose-a:no-underline hover:prose-a:underline
                                                prose-li:text-gray-700 dark:prose-li:text-slate-300 prose-strong:text-gray-800 dark:prose-strong:text-slate-200
                                                prose-table:border-gray-300 dark:prose-table:border-slate-700 prose-th:bg-gray-100 dark:prose-th:bg-slate-800 prose-th:text-gray-800 dark:prose-th:text-slate-200 prose-td:border-gray-300 dark:prose-td:border-slate-700"
                                                inner_html=body.clone()
                                            ></div>
                                        }
                                    })}

                                    <div class="mt-10 pt-6 border-t border-gray-200 dark:border-slate-800 space-y-6">
                                        {e.backlinks.clone().filter(|b| !b.is_empty()).map(|backlinks| {
                                            view! {
                                                <div>
                                                    <h4 class="text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 mb-3">{move || t("wiki.backlinks")}</h4>
                                                    <div class="flex flex-wrap gap-2">
                                                        {backlinks.iter().map(|bl| {
                                                            let p = bl.clone();
                                                            view! {
                                                                <button
                                                                    class="px-3 py-1 bg-gray-100 dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded text-xs text-gray-600 dark:text-slate-400
                                                                           cursor-pointer hover:border-teal-500 hover:text-teal-600 dark:hover:text-teal-400 transition-colors"
                                                                    on:click={
                                                                        let p = p.clone();
                                                                        move |_| select_entry(p.clone())
                                                                    }
                                                                >{bl.clone()}</button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </div>
                                            }
                                        })}

                                        {e.related.clone().filter(|r| !r.is_empty()).map(|related| {
                                            view! {
                                                <div>
                                                    <h4 class="text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 mb-3">{move || t("wiki.related")}</h4>
                                                    <div class="flex flex-wrap gap-2">
                                                        {related.iter().map(|r| {
                                                            let p = r.clone();
                                                            view! {
                                                                <button
                                                                    class="px-3 py-1 bg-gray-100 dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded text-xs text-gray-600 dark:text-slate-400
                                                                           cursor-pointer hover:border-teal-500 hover:text-teal-600 dark:hover:text-teal-400 transition-colors"
                                                                    on:click={
                                                                        let p = p.clone();
                                                                        move |_| select_entry(p.clone())
                                                                    }
                                                                >{r.clone()}</button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </div>
                                            }
                                        })}

                                        {e.contradictions.clone().filter(|c| !c.is_empty()).map(|contradictions| {
                                            view! {
                                                <div>
                                                    <h4 class="text-xs uppercase tracking-wider text-red-500/70 mb-3">{move || t("wiki.contradictions")}</h4>
                                                    <div class="flex flex-wrap gap-2">
                                                        {contradictions.iter().map(|c| {
                                                            let p = c.clone();
                                                            view! {
                                                                <button
                                                                    class="px-3 py-1 bg-red-950/20 border border-red-900/50 rounded text-xs text-red-400
                                                                           cursor-pointer hover:border-red-500 transition-colors"
                                                                    on:click={
                                                                        let p = p.clone();
                                                                        move |_| select_entry(p.clone())
                                                                    }
                                                                >{c.clone()}</button>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </div>
                                            }
                                        })}
                                    </div>

                                    <div class="mt-8">
                                        <button class="btn text-xs" on:click=move |_| {
                                            if show_concept.get() {
                                                show_concept.set(false);
                                                return;
                                            }
                                            if let Some(ref p) = active_path.get() {
                                                let p = p.clone();
                                                spawn_local(async move {
                                                    if let Ok(data) = api::fetch_concept_map(&p).await {
                                                        concept_map.set(data.mermaid);
                                                        show_concept.set(true);
                                                    }
                                                });
                                            }
                                        }>
                                            {move || t("wiki.concept_map")}
                                        </button>
                                    </div>

                                    <Show when=move || show_concept.get()>
                                        <div class="mt-4 p-4 bg-gray-50 dark:bg-slate-900 border border-gray-200 dark:border-slate-700 rounded-lg">
                                            <h4 class="text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 mb-3">Mermaid Concept Map</h4>
                                            <pre class="text-xs text-gray-600 dark:text-slate-400 whitespace-pre-wrap font-mono">
                                                {concept_map.get().unwrap_or_default()}
                                            </pre>
                                        </div>
                                    </Show>
                                </div>
                            }
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}