use leptos::prelude::*;

#[component]
pub fn DomainTags(domains: Signal<Vec<String>>) -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-2">
            {move || domains.get().iter().map(|d| {
                view! {
                    <span class="tag">{d.clone()}</span>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}