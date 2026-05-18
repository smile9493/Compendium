import { createI18n } from 'vue-i18n'
import zhCN from '@/locales/zh-CN.json'
import enUS from '@/locales/en-US.json'

const saved = typeof localStorage !== 'undefined' ? localStorage.getItem('rsut-locale') : null
const browser = typeof navigator !== 'undefined' ? navigator.language : 'zh-CN'
const locale = saved || (browser.startsWith('zh') ? 'zh-CN' : 'en-US')

export const i18n = createI18n({
  legacy: false,
  locale,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})

export function setLocale(lang) {
  i18n.global.locale.value = lang
  localStorage.setItem('rsut-locale', lang)
}
