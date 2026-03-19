# Thinking 气泡动画重设计

## 背景

byetype 当前的气泡窗口为每个任务状态使用不同的几何形状（方块、三角、菱形、八角形等），辨识度不够直观，也不够像 AI 产品。需要将其改为更具 AI 感的 "Thinking" 标签样式。

## 设计目标

- 让气泡看起来更像 AI 在思考
- 通过颜色区分不同处理阶段，便于定位卡在哪个环节
- 统一视觉风格，简洁现代

## 状态设计

### 进行中状态

| 状态 | 形态 | 颜色 | 内容 | 动画 |
|------|------|------|------|------|
| recording | 小圆形 (34x34) | 玫红 `#ff4757 → #ff6b81 → #c44569` | 白色 ● | 渐变色波 |
| transcribing | 药丸 (圆角矩形) | 靛紫 `#5f27cd → #6c63ff → #a29bfe` | 白色 "Thinking..." | 渐变色波 |
| optimizing | 药丸 (圆角矩形) | 湖蓝 `#0abde3 → #48dbfb → #74b9ff` | 白色 "Thinking..." | 渐变色波 |
| retrying | 药丸 (圆角矩形) | 琥珀 `#ff9f43 → #feca57 → #f39c12` | 白色 "Thinking..." | 渐变色波 |

### 终态

| 状态 | 形态 | 颜色 | 内容 |
|------|------|------|------|
| completed | 圆形 (34x34) | 绿色 `#2ecc71 → #27ae60` | 白色 ✓ |
| failed | 圆形 (34x34) | 灰色 `#95a5a6 → #7f8c8d` | 白色 ✕ |

### 流程

```
● (玫红) → Thinking... (靛紫) → Thinking... (湖蓝) → ✓ (绿) 或 ✕ (灰)
录音中       转录中              优化中           完成     失败
```

## 动画规格

所有进行中状态使用相同的**渐变色波**动画：

```css
background-size: 400% 100%;
animation: wave 4s ease infinite;

@keyframes wave {
  0%   { background-position: 0% 50%; }
  50%  { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}
```

- 渐变方向：`270deg`（从右到左流动）
- 每个状态使用 3-4 色渐变色带
- 动画周期：4s，ease 缓动，无限循环

## 尺寸变化

### 气泡窗口尺寸

- 当前：固定 40x40px
- 改为：recording/completed/failed 保持 40x40，transcribing/optimizing/retrying 需要扩大到约 130x36px 以容纳药丸文字

### 药丸样式

```css
.pill {
  display: inline-flex;
  align-items: center;
  padding: 7px 18px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  font-family: -apple-system, sans-serif;
  color: white;
}
```

## 涉及修改的文件

1. `bubble.html` — CSS 样式：移除旧的几何形状样式，新增药丸和圆形样式
2. `src/views/bubble/main.ts` — 更新 `shapeMap` 渲染逻辑
3. `src-tauri/src/bubble.rs` — 调整气泡窗口尺寸，药丸状态需要更宽的窗口

## 入场动画

保留现有的 `bounceIn` 弹入效果用于状态切换时的过渡。

```css
@keyframes bounceIn {
  0%   { transform: scale(0); opacity: 0; }
  50%  { transform: scale(1.18); }
  70%  { transform: scale(0.92); }
  100% { transform: scale(1); opacity: 1; }
}
```
