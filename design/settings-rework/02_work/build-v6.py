from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent
source = ROOT / '03_output/20261015-1213-设置界面v5-独立开关联动提示词.html'
output = ROOT / '03_output/20261015-1229-设置界面v6-优化开关与单独快捷键.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:100]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面v5</title>', '<title>ByeType设置界面v6</title>')
replace('<button class="btn-link" id="edit-prompts" data-opt-gated="" onclick="go(\'voice-prompts\')">编辑提示词</button><span class="mode-state" id="optimization-state">已开启</span>', '')
replace('<span contenteditable="true" aria-label="语音输入 1名称">语音输入 1</span>', '<span id="primary-voice-name" contenteditable="true" aria-label="语音输入名称">语音输入 1</span>')
replace('<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label"><span contenteditable="true" aria-label="语音输入 2名称">语音输入 2</span>', '<div class="setting-row" id="secondary-voice-row" data-opt-gated=""><div class="setting-row-main"><div class="setting-row-label"><span contenteditable="true" aria-label="语音输入 2名称">语音输入 2</span>')
replace("  document.getElementById('optimization-state').textContent = optimizationEnabled ? '已开启' : '已关闭';", "  document.getElementById('primary-voice-name').textContent = optimizationEnabled ? primaryVoiceName : '语音输入';")
replace('let optimizationEnabled = true;', "let optimizationEnabled = true;\nlet primaryVoiceName = '语音输入 1';")
replace('  optimizationEnabled = !!on;', "  if (optimizationEnabled) primaryVoiceName = document.getElementById('primary-voice-name').textContent.trim() || '语音输入 1';\n  optimizationEnabled = !!on;")
replace('onclick="go(\'voice-prompts\')">转写提示词</button>', 'onclick="go(\'voice-prompts\')"><span>转写提示词</span><span class="optimization-marker" aria-label="文本优化相关">优化</span></button>')
replace('<h2 class="content-title">转写提示词</h2>', '<h2 class="content-title optimization-heading">转写提示词<span class="optimization-marker">优化</span></h2>')
replace('<h2 class="content-title" style="margin-top:24px">文本优化提示词</h2>', '<h2 class="content-title optimization-heading" style="margin-top:24px">文本优化提示词<span class="optimization-marker">优化</span></h2>')
replace('</style>', '''
/* 优化相关入口的标记，与普通页和选中页状态区分。 */
.optimization-marker {
  display: inline-block;
  margin-left: 7px;
  padding: 1px 4px;
  border-radius: 3px;
  background: var(--accent-bg);
  color: var(--accent);
  font-size: 10px;
  font-weight: 500;
  line-height: 1.5;
  vertical-align: middle;
}
#prompt-navigation { border-left: 2px solid var(--accent); padding-left: 14px; }
.optimization-heading { border-left: 2px solid var(--accent); padding-left: 10px; }
</style>''')
# 仅首页开关和相关显示逻辑变动；输出稿保留完整其他页面。
assert 'optimization-state' not in html
assert 'edit-prompts' not in html
assert html.count('data-page="general"') == 2
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)
print(output)
