use leptos::prelude::*;

use crate::api::TreeNode;

#[component]
pub fn TreeView(
    node: Signal<Option<TreeNode>>,
    on_select: Callback<String>,
    active_path: Signal<Option<String>>,
) -> impl IntoView {
    let expanded = RwSignal::new(std::collections::HashSet::<String>::new());

    view! {
        <div class="overflow-y-auto">
            {move || {
                if let Some(ref tree) = node.get() {
                    if let Some(ref children) = tree.children {
                        render_children(
                            children,
                            0,
                            expanded,
                            on_select,
                            active_path,
                        ).into_any()
                    } else {
                        view! { <div class="text-xs text-gray-400 dark:text-slate-600 p-4">No entries</div> }.into_any()
                    }
                } else {
                    view! { <div class="text-xs text-gray-400 dark:text-slate-600 p-4">Loading...</div> }.into_any()
                }
            }}
        </div>
    }
}

fn render_children(
    children: &[TreeNode],
    depth: u32,
    expanded: RwSignal<std::collections::HashSet<String>>,
    on_select: Callback<String>,
    active_path: Signal<Option<String>>,
) -> impl IntoView {
    children
        .iter()
        .map(move |child| {
            let name = child.name.clone().unwrap_or_default();
            let is_entry = child.is_entry.unwrap_or(false);
            let path = child.path.clone();
            let has_children = child.children.as_ref().map(|c| !c.is_empty()).unwrap_or(false);

            if is_entry {
                let path_clone = path.clone().unwrap_or_default();
                let is_active = move || {
                    active_path.get().as_ref() == path.as_ref()
                };

                view! {
                    <div
                        class=move || format!(
                            "flex items-center gap-1.5 py-1.5 text-xs cursor-pointer hover:bg-teal-50 dark:hover:bg-teal-950/30 hover:text-teal-600 dark:hover:text-teal-400 transition-colors {}",
                            if is_active() { "text-teal-600 dark:text-teal-400 bg-teal-50 dark:bg-teal-950/20" } else { "text-gray-600 dark:text-slate-400" }
                        )
                        style=format!("padding-left:{}px", 16 + depth as usize * 16)
                        on:click={
                            let p = path_clone.clone();
                            move |_| on_select.run(p.clone())
                        }
                    >
                        <svg class="w-3.5 h-3.5 shrink-0 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                            <polyline points="14,2 14,8 20,8"/>
                        </svg>
                        <span class="truncate">{child.title.clone().unwrap_or(name.clone())}</span>
                    </div>
                }.into_any()
            } else {
                let name_clone1 = name.clone();
                let name_clone2 = name.clone();
                let name_clone3 = name.clone();

                let children_view = if has_children {
                    let sub = child.children.as_ref().unwrap();
                    let tv = render_children(sub, depth + 1, expanded, on_select, active_path);
                    view! {
                        <div class=move || {
                            if expanded.get().contains(&name_clone1) { "" } else { "hidden" }
                        }>
                            {tv}
                        </div>
                    }.into_any()
                } else {
                    view! { <div class="hidden"></div> }.into_any()
                };

                view! {
                    <div>
                        <div
                            class=format!("flex items-center gap-1.5 py-1.5 text-xs font-medium cursor-pointer hover:text-gray-700 dark:hover:text-slate-300 transition-colors text-gray-500 dark:text-slate-500")
                            style=format!("padding-left:{}px", 16 + depth as usize * 16)
                            on:click=move |_| {
                                expanded.update(|s| {
                                    if s.contains(&name_clone2) {
                                        s.remove(&name_clone2);
                                    } else {
                                        s.insert(name_clone2.clone());
                                    }
                                });
                            }
                        >
                            <svg class=move || format!("w-3.5 h-3.5 shrink-0 transition-transform {}", if expanded.get().contains(&name_clone3) { "rotate-90" } else { "" })
                                fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <polyline points="9,18 15,12 9,6"/>
                            </svg>
                            <span>{name}</span>
                        </div>
                        {children_view}
                    </div>
                }.into_any()
            }
        })
        .collect_view()
}