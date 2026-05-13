use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }
}

fn theme_storage_key() -> &'static str {
    "mcp-panel-theme"
}

fn load_theme() -> Theme {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(theme_storage_key()).ok().flatten())
            .map(|v| Theme::from_str(&v))
            .unwrap_or(Theme::Dark)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Theme::Dark
    }
}

fn save_theme(theme: Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = storage.set_item(theme_storage_key(), theme.as_str());
        }
    }
}

fn apply_theme_to_document(theme: Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let html_element = document.document_element().unwrap();
            let class_list = html_element.class_list();
            
            match theme {
                Theme::Dark => {
                    let _ = class_list.add_1("dark");
                }
                Theme::Light => {
                    let _ = class_list.remove_1("dark");
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct ThemeSignal {
    pub theme: RwSignal<Theme>,
}

impl ThemeSignal {
    pub fn new() -> Self {
        let theme = load_theme();
        apply_theme_to_document(theme);
        
        Self {
            theme: RwSignal::new(theme),
        }
    }

    pub fn toggle(&self) {
        let new_theme = match self.theme.get() {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
        self.theme.set(new_theme);
        save_theme(new_theme);
        apply_theme_to_document(new_theme);
    }
}

pub fn provide_theme() -> ThemeSignal {
    let s = ThemeSignal::new();
    provide_context(s);
    s
}

pub fn current_theme() -> Theme {
    use_context::<ThemeSignal>()
        .map(|s| s.theme.get())
        .unwrap_or(Theme::Dark)
}
