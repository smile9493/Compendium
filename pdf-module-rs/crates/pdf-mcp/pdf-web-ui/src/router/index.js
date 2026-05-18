import { createRouter, createWebHashHistory } from 'vue-router'
import { useWikiStore } from '@/stores/wiki'

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
    path: '/share/:token/:path(.*)',
    name: 'share-entry',
    component: () => import('@/views/ShareEntryView.vue'),
    props: true,
  },
]

const router = createRouter({
  history: createWebHashHistory('/app/'),
  routes,
})

router.beforeEach(async (to, from, next) => {
  if (!to.matched.length) {
    next({ path: '/' })
    return
  }

  const wikiStore = useWikiStore()

  if (to.name === 'share-entry') {
    next()
    return
  }
  if (to.name === 'entry' && to.params.path) {
    await wikiStore.loadEntry(to.params.path)
  } else if (to.name === 'wiki') {
    wikiStore.clearCurrentEntry()
  }

  next()
})

export default router
