import React from 'react'
import { createRoot } from 'react-dom/client'

function App() {
  return (
    <div style={{ padding: 20 }}>
      <h1>ByeType</h1>
      <p>Tauri v2 + React + TypeScript</p>
    </div>
  )
}

const container = document.getElementById('root')
if (container) {
  const root = createRoot(container)
  root.render(<App />)
}
