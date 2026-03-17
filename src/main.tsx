import { createRoot } from 'react-dom/client'
import { App } from './views/settings/App'
import './views/settings/index.css'

const container = document.getElementById('root')
if (container) {
  const root = createRoot(container)
  root.render(<App />)
}
