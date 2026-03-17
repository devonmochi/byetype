import React from 'react'

interface SettingGroupProps {
  title?: string
  children: React.ReactNode
}

export function SettingGroup({ title, children }: SettingGroupProps) {
  return (
    <div>
      {title && <div className="setting-group-title">{title}</div>}
      <div className="setting-group">
        {children}
      </div>
    </div>
  )
}
