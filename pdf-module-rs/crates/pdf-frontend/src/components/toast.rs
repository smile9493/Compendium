use leptos::prelude::*;

#[derive(Clone)]
pub struct ToastMsg {
    pub text: String,
    pub is_error: bool,
}

#[component]
pub fn Toast(toast: Signal<Option<ToastMsg>>) -> impl IntoView {
    view! {
        <div
            class="fixed top-6 right-6 z-[100] pointer-events-none"
        >
            <div
                class=move || {
                    let base = "px-5 py-3 rounded-lg text-sm transition-all duration-300 pointer-events-auto ";
                    if let Some(ref msg) = toast.get() {
                        if msg.is_error {
                            format!("{} bg-red-500/90 text-white opacity-100 translate-y-0", base)
                        } else {
                            format!("{} bg-green-500/90 text-black opacity-100 translate-y-0", base)
                        }
                    } else {
                        format!("{} opacity-0 -translate-y-4 pointer-events-none", base)
                    }
                }
            >
                {move || toast.get().map(|m| m.text).unwrap_or_default()}
            </div>
        </div>
    }
}

pub fn show_toast(text: String, is_error: bool) {
    if let Some(toast) = use_context::<RwSignal<Option<ToastMsg>>>() {
        toast.set(Some(ToastMsg {
            text: text.clone(),
            is_error,
        }));
        set_timeout(
            move || {
                if let Some(toast) = use_context::<RwSignal<Option<ToastMsg>>>() {
                    toast.set(None);
                }
            },
            std::time::Duration::from_secs(3),
        );
    }
}