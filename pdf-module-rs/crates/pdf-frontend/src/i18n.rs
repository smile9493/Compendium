use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }
}

#[derive(Clone)]
pub struct I18n {
    translations: HashMap<String, HashMap<&'static str, &'static str>>,
}

impl I18n {
    fn new() -> Self {
        let mut zh = HashMap::new();
        zh.insert("nav.dashboard", "仪表盘");
        zh.insert("nav.wiki", "Wiki 浏览");
        zh.insert("nav.config", "配置");
        zh.insert("nav.compile", "编译");
        zh.insert("nav.refresh", "刷新");
        zh.insert("nav.rebuild", "重建索引");
        zh.insert("stats.entries", "总条目数");
        zh.insert("stats.orphans", "孤立条目");
        zh.insert("stats.contradictions", "矛盾条目");
        zh.insert("stats.broken", "损坏链接");
        zh.insert("stats.idx_size", "索引大小");
        zh.insert("stats.quality", "平均质量");
        zh.insert("stats.nodes", "图谱节点");
        zh.insert("stats.edges", "边");
        zh.insert("stats.last_compile", "最后编译");
        zh.insert("stats.never", "从未");
        zh.insert("dashboard.title", "知识库仪表盘");
        zh.insert("dashboard.domains", "知识领域");
        zh.insert("dashboard.distribution", "领域分布");
        zh.insert("dashboard.empty", "点击刷新获取知识库健康报告");
        zh.insert("dashboard.report", "健康报告");
        zh.insert("wiki.title", "Wiki 浏览器");
        zh.insert("wiki.search", "搜索知识库...");
        zh.insert("wiki.concept_map", "概念图");
        zh.insert("wiki.empty", "选择条目开始阅读");
        zh.insert("wiki.empty_desc", "从左侧目录树选择一个知识条目");
        zh.insert("wiki.no_results", "无搜索结果");
        zh.insert("wiki.no_results_desc", "尝试其他关键词");
        zh.insert("wiki.backlinks", "反向链接");
        zh.insert("wiki.related", "相关条目");
        zh.insert("wiki.contradictions", "矛盾条目");
        zh.insert("wiki.source", "来源");
        zh.insert("wiki.version", "版本");
        zh.insert("wiki.quality", "质量");
        zh.insert("wiki.entries", "条目");
        zh.insert("wiki.domains", "领域");
        zh.insert("config.title", "运行时配置");
        zh.insert("config.key", "键");
        zh.insert("config.value", "值");
        zh.insert("config.actions", "操作");
        zh.insert("config.loading", "加载中...");
        zh.insert("config.empty", "无配置项");
        zh.insert("config.placeholder_key", "键（例如: vlm_api_key）");
        zh.insert("config.placeholder_value", "值");
        zh.insert("config.set", "设置");
        zh.insert("config.remove", "移除");
        zh.insert("compile.title", "编译状态");
        zh.insert("compile.running", "运行中");
        zh.insert("compile.yes", "是");
        zh.insert("compile.no", "否");
        zh.insert("compile.never", "从未");
        zh.insert("compile.outcome", "结果");
        zh.insert("compile.duration", "耗时");
        zh.insert("compile.message", "消息");
        zh.insert("compile.check", "检查状态");
        zh.insert("compile.trigger", "触发增量编译");
        zh.insert("compile.hint", "点击加载编译状态");
        zh.insert("toast.config_set", "配置已设置");
        zh.insert("toast.config_removed", "已移除");
        zh.insert("toast.compile_done", "编译完成");
        zh.insert("toast.rebuild_done", "索引已重建");
        zh.insert("toast.failed", "操作失败");
        zh.insert("toast.load_failed", "加载数据失败");
        zh.insert("toast.key_required", "请输入键");

        let mut en = HashMap::new();
        en.insert("nav.dashboard", "Dashboard");
        en.insert("nav.wiki", "Wiki");
        en.insert("nav.config", "Config");
        en.insert("nav.compile", "Compile");
        en.insert("nav.refresh", "Refresh");
        en.insert("nav.rebuild", "Rebuild Index");
        en.insert("stats.entries", "Total Entries");
        en.insert("stats.orphans", "Orphan Entries");
        en.insert("stats.contradictions", "Contradictions");
        en.insert("stats.broken", "Broken Links");
        en.insert("stats.idx_size", "Index Size");
        en.insert("stats.quality", "Avg Quality");
        en.insert("stats.nodes", "Graph Nodes");
        en.insert("stats.edges", "edges");
        en.insert("stats.last_compile", "Last Compile");
        en.insert("stats.never", "Never");
        en.insert("dashboard.title", "Knowledge Base Dashboard");
        en.insert("dashboard.domains", "Domains");
        en.insert("dashboard.distribution", "Domain Distribution");
        en.insert("dashboard.empty", "Click Refresh to load health report");
        en.insert("dashboard.report", "Health Report");
        en.insert("wiki.title", "Wiki Browser");
        en.insert("wiki.search", "Search knowledge base...");
        en.insert("wiki.concept_map", "Concept Map");
        en.insert("wiki.empty", "Select an entry to read");
        en.insert("wiki.empty_desc", "Choose a knowledge entry from the tree");
        en.insert("wiki.no_results", "No results");
        en.insert("wiki.no_results_desc", "Try different keywords");
        en.insert("wiki.backlinks", "Backlinks");
        en.insert("wiki.related", "Related");
        en.insert("wiki.contradictions", "Contradictions");
        en.insert("wiki.source", "Source");
        en.insert("wiki.version", "Version");
        en.insert("wiki.quality", "Quality");
        en.insert("wiki.entries", "entries");
        en.insert("wiki.domains", "domains");
        en.insert("config.title", "Runtime Configuration");
        en.insert("config.key", "Key");
        en.insert("config.value", "Value");
        en.insert("config.actions", "Actions");
        en.insert("config.loading", "Loading...");
        en.insert("config.empty", "No configuration entries");
        en.insert("config.placeholder_key", "Key (e.g. vlm_api_key)");
        en.insert("config.placeholder_value", "Value");
        en.insert("config.set", "Set");
        en.insert("config.remove", "Remove");
        en.insert("compile.title", "Compile Status");
        en.insert("compile.running", "Running");
        en.insert("compile.yes", "Yes");
        en.insert("compile.no", "No");
        en.insert("compile.never", "Never");
        en.insert("compile.outcome", "Outcome");
        en.insert("compile.duration", "Duration");
        en.insert("compile.message", "Message");
        en.insert("compile.check", "Check Status");
        en.insert("compile.trigger", "Trigger Incremental Compile");
        en.insert("compile.hint", "Click to load compile status");
        en.insert("toast.config_set", "Config set");
        en.insert("toast.config_removed", "Removed");
        en.insert("toast.compile_done", "Compile completed");
        en.insert("toast.rebuild_done", "Index rebuilt");
        en.insert("toast.failed", "Operation failed");
        en.insert("toast.load_failed", "Failed to load data");
        en.insert("toast.key_required", "Key required");

        let mut translations = HashMap::new();
        translations.insert("zh".to_string(), zh);
        translations.insert("en".to_string(), en);
        Self { translations }
    }

    pub fn t(&self, lang: Lang, key: &str) -> String {
        self.translations
            .get(lang.as_str())
            .and_then(|map| map.get(key))
            .unwrap_or(&key)
            .to_string()
    }
}

fn i18n_instance() -> &'static I18n {
    static INSTANCE: std::sync::OnceLock<I18n> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(I18n::new)
}

fn lang_storage_key() -> &'static str {
    "mcp-panel-lang"
}

fn load_lang() -> Lang {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(lang_storage_key()).ok().flatten())
            .and_then(|v| match v.as_str() {
                "en" => Some(Lang::En),
                _ => Some(Lang::Zh),
            })
            .unwrap_or(Lang::Zh)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Lang::Zh
    }
}

fn save_lang(lang: Lang) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = s.set_item(lang_storage_key(), lang.as_str());
        }
    }
}

use leptos::prelude::*;

#[derive(Clone, Copy)]
pub struct LangSignal {
    pub lang: RwSignal<Lang>,
}

impl LangSignal {
    pub fn new() -> Self {
        Self {
            lang: RwSignal::new(load_lang()),
        }
    }
}

pub fn provide_i18n() -> LangSignal {
    let s = LangSignal::new();
    provide_context(s);
    s
}

pub fn current_lang() -> Lang {
    use_context::<LangSignal>()
        .map(|s| s.lang.get())
        .unwrap_or(Lang::Zh)
}

pub fn t(key: &str) -> String {
    let lang = current_lang();
    i18n_instance().t(lang, key)
}

pub fn use_t() -> fn(&str) -> String {
    t
}