import { onMounted, onBeforeUnmount } from 'vue'
import { useSearchStore } from '@/stores/search'

export function useKeyboard(searchInputRef) {
  const searchStore = useSearchStore()

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
