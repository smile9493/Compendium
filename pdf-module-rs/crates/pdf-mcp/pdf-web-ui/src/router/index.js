import { createRouter, createWebHashHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    name: 'wiki',
    component: () => import('@/views/WikiBrowser.vue'),
  },
  {
    path: '/wiki/:path(.*)',
    name: 'entry',
    component: () => import('@/views/EntryDetail.vue'),
    props: true,
  },
  {
    path: '/search',
    name: 'search',
    component: () => import('@/views/SearchResults.vue'),
  },
]

const router = createRouter({
  history: createWebHashHistory('/app/'),
  routes,
})

export default router
