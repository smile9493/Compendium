use leptos::prelude::*;

#[component]
pub fn StatCard(
    label: String,
    value: Signal<String>,
    detail: Option<Signal<String>>,
    #[prop(optional)] accent: Option<&'static str>,
) -> impl IntoView {
    let color_class = match accent {
        Some("green") => "text-green-600 dark:text-green-400",
        Some("yellow") => "text-yellow-600 dark:text-yellow-400",
        Some("red") => "text-red-600 dark:text-red-400",
        Some("blue") => "text-blue-600 dark:text-blue-400",
        _ => "text-gray-900 dark:text-slate-100",
    };

    view! {
        <div class="card">
            <div class="text-xs uppercase tracking-wider text-gray-500 dark:text-slate-500 mb-2">{label}</div>
            <div class=move || format!("text-2xl font-bold {}", color_class)>
                {value.get()}
            </div>
            {detail.map(|d| view! { <div class="text-xs text-gray-500 dark:text-slate-500 mt-1">{move || d.get()}</div> })}
        </div>
    }
}