import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'
import './styles/tokens.css'
import './styles/layout.css'
import './styles/main.css'
import tooltipDirective from './directives/tooltip'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.directive('tooltip', tooltipDirective)
app.mount('#app')
