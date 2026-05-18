import { ref, watch, onBeforeUnmount } from 'vue'

export function useScrollSpy(scrollElRef, proseSelector = '.prose') {
  const activeHeadingIdx = ref(-1)
  let observer = null

  function setup() {
    cleanup()
    const el = scrollElRef?.value
    if (!el) return

    observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const idx = parseInt(entry.target.dataset.headingIdx || '-1')
            if (idx >= 0) activeHeadingIdx.value = idx
          }
        }
      },
      { root: el, rootMargin: '-48px 0px -70% 0px', threshold: 0 }
    )

    const proseEl = el.querySelector(proseSelector)
    if (!proseEl) return

    const hs = proseEl.querySelectorAll('h1, h2, h3, h4')
    hs.forEach((h, i) => {
      h.dataset.headingIdx = String(i)
      observer.observe(h)
    })
  }

  function cleanup() {
    if (observer) {
      observer.disconnect()
      observer = null
    }
  }

  function reset() {
    activeHeadingIdx.value = -1
  }

  onBeforeUnmount(() => {
    cleanup()
  })

  return { activeHeadingIdx, setup, reset }
}