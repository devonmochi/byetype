# Thinking 气泡动画重设计

## 背景

byetype 当前的气泡窗口为每个任务状态使用不同的几何形状（方块、三角、菱形、八角形等），辨识度不够直观，也不够像 AI 产品。需要将其改为更具 AI 感的 "Thinking" 标签样式。

## 设计目标

- 让气泡看起来更像 AI 在思考
- 通过颜色区分不同处理阶段，便于定位卡在哪个环节
- 统一视觉风格，简洁现代

## 状态设计

### 进行中状态

| 状态 | 形态 | 内容 | 动画 |
|------|------|------|------|
| recording | 圆形 34x34 | 白色 ● (U+25CF, 16px) | 玫红渐变色波 |
| transcribing | 药丸 ~120x32 | 白色 "Thinking..." (13px, 600) | 靛紫渐变色波 |
| optimizing | 药丸 ~120x32 | 白色 "Thinking..." (13px, 600) | 湖蓝渐变色波 |
| retrying | 药丸 ~120x32 | 白色 "Thinking..." (13px, 600) | 琥珀渐变色波 |

> **注意**：`retrying` 在前端 types.ts 中已定义，但当前后端重试时发送的是 `transcribing`/`optimizing` 状态。CSS 需要定义 retrying 样式以备未来使用，但当前不会被触发。

### 终态

| 状态 | 形态 | 内容 |
|------|------|------|
| completed | 圆形 34x34 | 白色 ✓ (U+2713, 18px, bold) |
| failed | 圆形 34x34 | 白色 ✕ (U+2715, 16px, bold) |

### 任务编号

新设计**移除任务编号**，不在气泡中显示数字。并发多任务时通过颜色区分当前阶段，不区分具体任务。

### 流程

```
● (玫红) → Thinking... (靛紫) → Thinking... (湖蓝) → ✓ (绿) 或 ✕ (灰)
录音中       转录中              优化中           完成     失败
```

## 动画规格

所有进行中状态使用相同的**渐变色波**动画，仅颜色不同：

```css
@keyframes wave {
  0%   { background-position: 0% 50%; }
  50%  { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}
```

各状态完整 CSS（渐变需 4 色以确保 `background-size: 400%` 下无缝循环）：

```css
.s-recording {
  width: 34px; height: 34px; border-radius: 50%;
  background: linear-gradient(270deg, #ff4757, #ff6b81, #ff4757, #c44569);
  background-size: 400% 100%;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both,
             wave 4s ease infinite 0.6s;
  box-shadow: 0 4px 14px rgba(255,71,87,0.4);
  display: flex; align-items: center; justify-content: center;
  color: white; font-size: 16px;
}

.s-transcribing {
  display: inline-flex; align-items: center;
  padding: 7px 18px; border-radius: 20px;
  background: linear-gradient(270deg, #5f27cd, #6c63ff, #a29bfe, #5f27cd);
  background-size: 400% 100%;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both,
             wave 4s ease infinite 0.6s;
  box-shadow: 0 4px 14px rgba(108,99,255,0.4);
  color: white; font-size: 13px; font-weight: 600;
  font-family: -apple-system, sans-serif;
}

.s-optimizing {
  display: inline-flex; align-items: center;
  padding: 7px 18px; border-radius: 20px;
  background: linear-gradient(270deg, #0abde3, #48dbfb, #74b9ff, #0abde3);
  background-size: 400% 100%;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both,
             wave 4s ease infinite 0.6s;
  box-shadow: 0 4px 14px rgba(10,189,227,0.4);
  color: white; font-size: 13px; font-weight: 600;
  font-family: -apple-system, sans-serif;
}

.s-retrying {
  display: inline-flex; align-items: center;
  padding: 7px 18px; border-radius: 20px;
  background: linear-gradient(270deg, #ff9f43, #feca57, #f39c12, #ff9f43);
  background-size: 400% 100%;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both,
             wave 4s ease infinite 0.6s;
  box-shadow: 0 4px 14px rgba(255,159,67,0.4);
  color: white; font-size: 13px; font-weight: 600;
  font-family: -apple-system, sans-serif;
}

.s-completed {
  width: 34px; height: 34px; border-radius: 50%;
  background: linear-gradient(135deg, #2ecc71, #27ae60);
  box-shadow: 0 4px 16px rgba(46,204,113,0.5);
  display: flex; align-items: center; justify-content: center;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both;
}

.s-failed {
  width: 34px; height: 34px; border-radius: 50%;
  background: linear-gradient(135deg, #95a5a6, #7f8c8d);
  box-shadow: 0 2px 8px rgba(149,165,166,0.4);
  display: flex; align-items: center; justify-content: center;
  animation: bounceIn 0.6s cubic-bezier(0.34,1.56,0.64,1) both;
}
```

旧的 `recPulse` 和 `retryPulse` 动画移除，统一用 `wave` 替代。

## 入场动画

保留 `bounceIn`，用于每次状态切换时的弹入效果。状态间的切换是**硬切**（旧 DOM 替换 + 新元素 bounceIn），不做形状过渡动画。

```css
@keyframes bounceIn {
  0%   { transform: scale(0); opacity: 0; }
  50%  { transform: scale(1.18); }
  70%  { transform: scale(0.92); }
  100% { transform: scale(1); opacity: 1; }
}
```

## 窗口尺寸策略

**方案：预创建时使用最大尺寸，CSS 负责内部布局。**

当前窗口固定 40x40 + `resizable(false)`，药丸状态需要 ~120px 宽度。

- 将 `BUBBLE_SIZE` 改为宽高分离：`BUBBLE_WIDTH = 140.0`, `BUBBLE_HEIGHT = 44.0`
- `inner_size(BUBBLE_WIDTH, BUBBLE_HEIGHT)` 预创建所有窗口
- 窗口始终透明背景，超出可视区域的部分自然不可见
- `#bubble` 容器改为 `width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;`
- 圆形状态（recording/completed/failed）在 140x44 窗口中居中显示 34x34 圆形
- 药丸状态自然撑开到 ~120x32，居中于窗口

这样避免了运行时 `set_size` 可能带来的闪烁，代码改动量最小。

## 窗口定位

当前 `OFFSET_X=10, OFFSET_Y=10` 从光标位置偏移。窗口加宽到 140px 后：

- 保持从光标右下角偏移的策略不变
- `OFFSET_X` 和 `OFFSET_Y` 无需修改
- 药丸在窗口内居中，视觉上对齐合理

## 涉及修改的文件

### bubble.html

- 移除所有旧几何形状 CSS（`.s-recording` 旋转方块、`.s-transcribing` 三角形、`.s-optimizing` 菱形、`.s-retrying` 钻石、`.s-failed` 八角形）
- 移除旧动画（`bounceInRotate`、`bounceInDiamond`、`recPulse`、`retryPulse`）
- 新增 `.s-recording` 圆形、药丸类的完整 CSS
- 新增 `wave` 动画
- `#bubble` 容器改为 `width: 100%; height: 100%`
- `.check` 和 `.x` 文字样式保留，`.num` 移除

### src/views/bubble/main.ts

- 更新 `shapeMap`：
  - `recording` → `<div class="s-recording">●</div>`
  - `transcribing` → `<div class="s-transcribing">Thinking...</div>`
  - `optimizing` → `<div class="s-optimizing">Thinking...</div>`
  - `retrying` → `<div class="s-retrying">Thinking...</div>`
  - `completed` → `<div class="s-completed"><span class="check">✓</span></div>`
  - `failed` → `<div class="s-failed"><span class="x">✕</span></div>`
- `render` 函数签名中 `taskNumber` 参数保留但不再使用（保持接口兼容）

### src-tauri/src/bubble.rs

- `BUBBLE_SIZE` → `BUBBLE_WIDTH = 140.0` + `BUBBLE_HEIGHT = 44.0`
- `inner_size(BUBBLE_WIDTH, BUBBLE_HEIGHT)`
- 其他逻辑不变
