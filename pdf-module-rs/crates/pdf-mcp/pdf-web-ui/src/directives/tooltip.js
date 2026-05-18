const tooltip = {
  mounted(el, binding) {
    const tooltip = document.createElement('div')
    tooltip.className = 'tooltip'
    tooltip.textContent = binding.value
    document.body.appendChild(tooltip)

    el._tooltip = tooltip
    let timeoutId = null

    el.addEventListener('mouseenter', () => {
      if (timeoutId) clearTimeout(timeoutId)
      timeoutId = setTimeout(() => {
        if (!el._tooltip) return
        const rect = el.getBoundingClientRect()
        const tRect = tooltip.getBoundingClientRect()
        const x = rect.left + rect.width / 2 - tRect.width / 2
        const y = rect.top - tRect.height - 6

        tooltip.style.left = `${Math.max(8, Math.min(x, window.innerWidth - tRect.width - 8))}px`
        tooltip.style.top = `${Math.max(8, y)}px`
        tooltip.classList.add('visible')
      }, 400)
    })

    el.addEventListener('mouseleave', () => {
      if (timeoutId) {
        clearTimeout(timeoutId)
        timeoutId = null
      }
      tooltip.classList.remove('visible')
    })
  },
  unmounted(el) {
    if (el._tooltip) {
      el._tooltip.remove()
      delete el._tooltip
    }
  },
}

export default tooltip