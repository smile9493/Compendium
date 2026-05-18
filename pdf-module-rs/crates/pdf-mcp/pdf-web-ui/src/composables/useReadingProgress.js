import { ref, onBeforeUnmount } from 'vue'

export function useReadingProgress(scrollContainerSelector = '.app-main') {
  const progress = ref(0)
  let scrollEl = null

  function onScroll() {
    if (!scrollEl) return
    const scrollTop = scrollEl.scrollTop
    const scrollHeight = scrollEl.scrollHeight - scrollEl.clientHeight
    progress.value = scrollHeight > 0 ? Math.min(scrollTop / scrollHeight, 1) : 0
  }

  function setup() {
    cleanup()
    scrollEl = document.querySelector(scrollContainerSelector)
    if (scrollEl) {
      scrollEl.addEventListener('scroll', onScroll, { passive: true })
      onScroll()
    }
  }

  function cleanup() {
    if (scrollEl) {
      scrollEl.removeEventListener('scroll', onScroll)
      scrollEl = null
    }
  }

  onBeforeUnmount(() => {
    cleanup()
  })

  return { progress, setup, cleanup }
}