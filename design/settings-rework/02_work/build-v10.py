from pathlib import Path

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1300-设置界面v9-语音识别与强化规则.html'
output = root / '03_output/20261015-1307-设置界面v10-强化输入缩进.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:120]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面对齐稿v9</title>', '<title>ByeType设置界面对齐稿v10</title>')
primary_start = html.index('<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label"><span id="primary-voice-name"')
mode_start = html.index('<div class="setting-row" id="optimization-row">', primary_start)
next_start = html.index('<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label">按住说话模式', mode_start)
input_rows = html[primary_start:mode_start]
mode_row = html[mode_start:next_start]
assert input_rows.count('id="primary-voice-name"') == 1
assert input_rows.count('id="secondary-voice-row"') == 1
input_rows = input_rows.replace('<div class="setting-row">', '<div class="setting-row" id="primary-voice-row">', 1)
layout = '<div id="plain-voice-slot" hidden></div>' + mode_row + '<div id="strengthened-voice-inputs" class="strengthened-voice-inputs" data-opt-gated="">' + input_rows + '</div>'
html = html[:primary_start] + layout + html[next_start:]
replace('</style>', '''
/* 开启时两个语音入口属于文本强化，关闭时单独入口回到开关上方。 */
.strengthened-voice-inputs { margin-left: 32px; padding-left: 0; margin-bottom: 4px; }
.strengthened-voice-inputs .setting-row { padding-left: 16px; }
</style>''')
replace("  document.querySelectorAll('[data-opt-gated]').forEach(el => { el.hidden = !optimizationEnabled; });", "  document.querySelectorAll('[data-opt-gated]').forEach(el => { el.hidden = !optimizationEnabled; });\n  syncVoiceInputs();")
replace('function toggle(btn) {', '''function syncVoiceInputs() {
  const row = document.getElementById('primary-voice-row');
  const plainSlot = document.getElementById('plain-voice-slot');
  const strengthenedSlot = document.getElementById('strengthened-voice-inputs');
  plainSlot.hidden = optimizationEnabled;
  if (optimizationEnabled) {
    strengthenedSlot.insertBefore(row, document.getElementById('secondary-voice-row'));
  } else {
    plainSlot.appendChild(row);
  }
}
function toggle(btn) {''')
replace("go('general');\n</script>", "go('general');\nsyncVoiceInputs();\n</script>")
assert not output.exists(), '不覆盖已交付对齐稿'
output.write_text(html)
print(output)
