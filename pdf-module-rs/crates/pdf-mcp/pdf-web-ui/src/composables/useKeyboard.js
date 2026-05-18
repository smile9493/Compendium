import { onMounted, onBeforeUnmount, ref } from 'vue'
import { useSearchStore } from '@/stores/search'
import { useWikiStore } from '@/stores/wiki'

export function useKeyboard(searchInputRef) {
  const searchStore = useSearchStore()
  const wikiStore = useWikiStore()

  function onGlobalKeydown(e) {
    if (e.key === 'Escape' && !e.target.closest('.dialog') && !e.target.closest('input, textarea, select')) {
      if (searchStore.open) {
        e.preventDefault()
        searchStore.close()
        return
      }
    }

    if (e.key === '/' && !e.ctrlKey && !e.metaKey && document.activeElement !== searchInputRef?.value) {
      e.preventDefault()
      searchInputRef?.value?.focus()
      searchInputRef?.value?.select()
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', onGlobalKeydown)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', onGlobalKeydown)
  })
}