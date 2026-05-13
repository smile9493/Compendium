use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::domain_tags::DomainTags;
use crate::components::pie_chart::PieChart;
use crate::components::stat_card::StatCard;
use crate::components::toast::show_toast;
use crate::i18n::use_t;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let t = use_t();
    let health = RwSignal::new(None::<api::HealthData>);
    let loading = RwSignal::new(false);

    loading.set(true);
    spawn_local(async move {
        match api::fetch_health().await {
            Ok(data) => health.set(Some(data)),
            Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
        }
        loading.set(false);
    });

    let generated = move || {
        health.with(|h| {
            h.as_ref()
                .and_then(|d| d.generated_at.as_ref())
                .map(|ts| format!("Generated: {}", ts))
                .unwrap_or_default()
        })
    };

    view! {
        <div>
            <div class="flex items-center justify-between mb-6">
                <div>
                    <h1 class="text-xl font-semibold">{move || t("dashboard.title")}</h1>
                    <p class="text-xs text-gray-500 dark:text-slate-500 mt-1">{generated}</p>
                </div>
                <div class="flex gap-2">
                    <button class="btn" on:click=move |_| {
                        loading.set(true);
                        spawn_local(async move {
                            match api::fetch_health().await {
                                Ok(data) => health.set(Some(data)),
                                Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
                            }
                            loading.set(false);
                        });
                    }>
                        {move || if loading.get() { String::from("...") } else { t("nav.refresh") }}
                    </button>
                    <button class="btn btn-primary" on:click=move |_| {
                        spawn_local(async move {
                            match api::rebuild_index().await {
                                Ok(data) => {
                                    let count = data.fulltext_entries_indexed.unwrap_or(0);
                                    show_toast(format!("{}: {} {}", t("toast.rebuild_done"), count, t("wiki.entries")), false);
                                    loading.set(true);
                                    spawn_local(async move {
                                        match api::fetch_health().await {
                                            Ok(data) => health.set(Some(data)),
                                            Err(e) => show_toast(format!("{}: {}", t("toast.load_failed"), e), true),
                                        }
                                        loading.set(false);
                                    });
                                }
                                Err(e) => show_toast(format!("{}: {}", t("toast.failed"), e), true),
                            }
                        });
                    }>
                        {move || t("nav.rebuild")}
                    </button>
                </div>
            </div>

            <Show
                when=move || health.get().is_some()
                fallback=move || view! {
                    <div class="text-center py-20 text-gray-500 dark:text-slate-500">
                        <p class="text-sm">{move || t("dashboard.empty")}</p>
                    </div>
                }
            >
                <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
                    {move || view! {
                        <StatCard
                            label=t("stats.entries")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.total_entries).map(|v| v.to_string()).unwrap_or_else(|| "--".into())))
                            detail=None
                            accent="blue"
                        />
                        <StatCard
                            label=t("stats.orphans")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.orphan_count).map(|v| v.to_string()).unwrap_or_else(|| "--".into())))
                            detail=None
                            accent="yellow"
                        />
                        <StatCard
                            label=t("stats.contradictions")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.contradiction_count).map(|v| v.to_string()).unwrap_or_else(|| "--".into())))
                            detail=None
                            accent="red"
                        />
                        <StatCard
                            label=t("stats.broken")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.broken_link_count).map(|v| v.to_string()).unwrap_or_else(|| "--".into())))
                            detail=None
                            accent="yellow"
                        />
                        <StatCard
                            label=t("stats.idx_size")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.index_size_mb).map(|v| format!("{} MB", v)).unwrap_or_else(|| "--".into())))
                            detail=None
                        />
                        <StatCard
                            label=t("stats.quality")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.avg_quality_score.clone()).unwrap_or_else(|| "--".into())))
                            detail=None
                            accent="green"
                        />
                        <StatCard
                            label=t("stats.nodes")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.graph_nodes).map(|v| v.to_string()).unwrap_or_else(|| "--".into())))
                            detail=Some(Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.graph_edges).map(|v| format!("{} {}", v, t("stats.edges"))).unwrap_or_default())))
                        />
                        <StatCard
                            label=t("stats.last_compile")
                            value=Signal::derive(move || health.with(|h| h.as_ref().and_then(|d| d.last_compile.clone()).unwrap_or_else(|| t("stats.never"))))
                            detail=None
                        />
                    }}
                </div>

                <div class="card mb-4">
                    <h2 class="text-sm font-semibold mb-3">{move || t("dashboard.domains")}</h2>
                    <DomainTags domains=Signal::derive(move || {
                        health.with(|h| h.as_ref().and_then(|d| d.domains.clone()).unwrap_or_default())
                    })/>
                </div>

                <div class="card mb-4">
                    <h2 class="text-sm font-semibold mb-3">{move || t("dashboard.distribution")}</h2>
                    <PieChart domains=Signal::derive(move || {
                        health.with(|h| h.as_ref().and_then(|d| d.domains.clone()).unwrap_or_default())
                    })/>
                </div>

                {move || {
                    health.with(|h| {
                        h.as_ref().and_then(|d| d.report_text.clone()).map(|report| {
                            view! {
                                <div class="card">
                                    <h2 class="text-sm font-semibold mb-3">{t("dashboard.report")}</h2>
                                    <pre class="text-xs text-gray-600 dark:text-slate-400 whitespace-pre-wrap font-mono">{report}</pre>
                                </div>
                            }
                        })
                    })
                }}
            </Show>
        </div>
    }
}