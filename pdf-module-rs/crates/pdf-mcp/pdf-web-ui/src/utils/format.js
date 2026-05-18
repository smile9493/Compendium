export function formatQuality(score) {
  if (score == null) return '-'
  if (typeof score === 'string') return score
  return `${(score * 100).toFixed(0)}%`
}