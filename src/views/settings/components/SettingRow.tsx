import React from 'react'

interface SettingRowProps {
  label: React.ReactNode
  description?: string
  children: React.ReactNode
}

export function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="setting-row">
      <div>
        <div className="setting-row-label">{label}</div>
        {description && <div className="setting-row-description">{description}</div>}
      </div>
      <div style={{ flexShrink: 0 }}>
        {children}
      </div>
    </div>
  )
}
