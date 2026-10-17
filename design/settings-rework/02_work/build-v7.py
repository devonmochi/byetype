from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
source = ROOT / '03_output/20261015-1229-设置界面v6-优化开关与单独快捷键.html'
output = ROOT / '03_output/20261015-1242-设置界面v7-文本强化.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:100]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面v6</title>', '<title>ByeType设置界面对齐稿v7</title>')
# 将独立开关移到主语音输入之后、第二语音输入之前。
start = html.index('<div class="setting-row" id="optimization-row">')
primary = '<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label"><span id="primary-voice-name"'
end = html.index(primary, start)
mode_row = html[start:end]
assert mode_row.count('id="optimization-switch"') == 1
html = html[:start] + html[end:]
secondary = '<div class="setting-row" id="secondary-voice-row"'
assert html.count(secondary) == 1
html = html.replace(secondary, mode_row + secondary, 1)
replace('<div class="setting-row-label mode-label">文本优化</div>', '<div class="setting-row-label mode-label">文本强化<span class="strengthening-formula">（语音识别文本＋强化规则＝最终文本）</span></div>')
replace('aria-label="文本优化" aria-checked="true"', 'aria-label="文本强化" aria-checked="true"')
# 统一新增功能的名称，保留原有转写提示词名称和真实提示词正文。
html = html.replace('aria-label="文本优化相关"', 'aria-label="文本强化相关"')
html = html.replace('class="optimization-marker" aria-label="文本强化相关">优化</span>', 'class="optimization-marker" aria-label="文本强化相关">强化</span>')
html = html.replace('class="optimization-marker">优化</span>', 'class="optimization-marker">强化</span>')
html = html.replace('>文本优化提示词<span', '>文本强化提示词<span')
html = html.replace('aria-label="文本优化提示词"', 'aria-label="文本强化提示词"')
html = html.replace('>文本优化模型</div>', '>文本强化模型</div>')
replace('</style>', '\n.strengthening-formula { font-size: 11px; font-weight: 400; color: var(--text-secondary); margin-left: 5px; line-height: 1.6; }\n</style>')
assert html.index('id="primary-voice-name"') < html.index('id="optimization-switch"') < html.index('id="secondary-voice-row"')
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

old_check = ROOT / '02_work/check-v6.cjs'
check = old_check.read_text()
check = check.replace('20261015-1229-设置界面v6-优化开关与单独快捷键.html', output.name)
check = check.replace("assert(home.indexOf('id=\"optimization-switch\"') < home.indexOf('id=\"shortcut-template\"'));", "assert(home.indexOf('id=\"primary-voice-name\"') < home.indexOf('id=\"optimization-switch\"'));\nassert(home.indexOf('id=\"optimization-switch\"') < home.indexOf('id=\"secondary-voice-row\"'));\nassert(home.includes('文本强化'));\nassert(home.includes('（语音识别文本＋强化规则＝最终文本）'));\nassert(!home.includes('已开启') && !home.includes('已关闭'));\nassert(html.includes('class=\"optimization-marker\">强化</span>'));")
check = check.replace('通过：开关仅保留按钮、', '通过：语音输入在前、文本强化开关在后、公式文案一致、强化入口标记、开关仅保留按钮、')
new_check = ROOT / '02_work/check-v7.cjs'
assert not new_check.exists(), '不覆盖检查脚本'
new_check.write_text(check)
print(output)
