from pathlib import Path

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1349-设置界面v12-双向规则增强.html'
output = root / '03_output/20261015-1401-设置界面v13-固定语音入口名称.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:100]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面对齐稿v12</title>', '<title>ByeType设置界面对齐稿v13</title>')
replace('<span id="primary-voice-name" contenteditable="true" aria-label="语音输入名称">语音输入 1</span>', '<span id="primary-voice-name">语音输入 1</span>')
replace('<span contenteditable="true" aria-label="语音输入 2名称">语音输入 2</span>', '<span id="secondary-voice-name">语音输入 2</span>')
replace("let primaryVoiceName = '语音输入 1';\n", '')
replace("  if (optimizationEnabled) primaryVoiceName = document.getElementById('primary-voice-name').textContent.trim() || '语音输入 1';\n", '')
replace("optimizationEnabled ? primaryVoiceName : '语音输入'", "optimizationEnabled ? '语音输入 1' : '语音输入'")
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

check = (root / '02_work/check-v12.cjs').read_text().replace(source.name, output.name)
check = check.replace("get('primary-voice-name').textContent='工作语音';\n", '')
check = check.replace("get('primary-voice-name').textContent,'工作语音'", "get('primary-voice-name').textContent,'语音输入 1'")
check += "\nassert.equal(get('primary-voice-name').getAttribute('contenteditable'),undefined);\nassert.equal(get('secondary-voice-name').getAttribute('contenteditable'),undefined);\nassert(!script.includes('primaryVoiceName'));\nconsole.log('通过：语音输入1、语音输入2名称固定且不可编辑，关闭时仍显示语音输入。');\n"
check_path = root / '02_work/check-v13.cjs'
assert not check_path.exists()
check_path.write_text(check)
print(output)
