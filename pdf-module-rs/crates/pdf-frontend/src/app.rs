use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, Title};
use leptos_router::{
    components::{Outlet, ParentRoute, Route, Router, Routes},
    path,
};

use crate::components::layout::AppLayout;
use crate::pages::{compile::CompilePage, config::ConfigPage, dashboard::DashboardPage, wiki::WikiPage};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Title text="rsut-pdf-mcp"/>

        <Router>
            <Routes fallback=|| view! { <NotFound/> }>
                <ParentRoute path=path!("/") view=AppLayout>
                    <Route path=path!("") view=DashboardPage/>
                    <Route path=path!("wiki") view=WikiPage/>
                    <Route path=path!("wiki/:domain/*path") view=WikiPage/>
                    <Route path=path!("config") view=ConfigPage/>
                    <Route path=path!("compile") view=CompilePage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center h-64 text-gray-500 dark:text-slate-500">
            <h2 class="text-xl font-semibold text-gray-700 dark:text-slate-300 mb-2">404</h2>
            <p class="text-sm">Page not found</p>
        </div>
    }
}