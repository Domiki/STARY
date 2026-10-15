import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import { App } from './app/App'
import { initI18n } from './i18n'
import { loadLanguagePreference } from './features/settings/language-preference'
import './styles/index.css'

void initI18n(loadLanguagePreference()).then(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})
