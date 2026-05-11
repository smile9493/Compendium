use leptos::prelude::*;

#[component]
pub fn PieChart(domains: Signal<Vec<String>>) -> impl IntoView {
    let colors = [
        "#3b82f6", "#22c55e", "#eab308", "#ef4444", "#a855f7", "#ec4899", "#14b8a6",
        "#f97316", "#6366f1", "#84cc16",
    ];

    view! {
        <div class="flex gap-5 flex-wrap">
            <div class="w-[180px] h-[180px] shrink-0">
                <svg viewBox="0 0 36 36" class="w-full h-full">
                    {move || {
                        let list = domains.get();
                        let n = list.len();
                        if n == 0 {
                            view! { <circle cx="18" cy="18" r="15.9" fill="none" stroke="#334155" stroke-width="3"/> }
                                .into_any()
                        } else {
                            let pct = 100.0 / n as f64;
                            let segments = list.iter().enumerate().map(|(i, _)| {
                                let offset = i as f64 * pct;
                                let color = colors[i % colors.len()];
                                let dash = format!("{} {}", pct, 100.0 - pct);
                                view! {
                                    <circle cx="18" cy="18" r="15.9" fill="none" stroke=color
                                        stroke-width="3" stroke-dasharray=dash
                                        stroke-dashoffset=format!("-{}", offset)
                                        transform="rotate(-90 18 18)"/>
                                }
                            }).collect_view();
                            segments.into_any()
                        }
                    }}
                </svg>
            </div>
            <div class="flex flex-col gap-1.5 justify-center">
                {move || domains.get().iter().enumerate().map(|(i, d)| {
                    let color = colors[i % colors.len()];
                    view! {
                        <div class="flex items-center gap-2 text-xs">
                            <div class="w-3 h-3 rounded-sm shrink-0" style=format!("background:{}", color)></div>
                            <span class="text-gray-700 dark:text-slate-300">{d.clone()}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}