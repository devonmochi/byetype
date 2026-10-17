from pathlib import Path

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1251-设置界面v8-拆分转录与强化提示词.html'
output = root / '03_output/20261015-1300-设置界面v9-语音识别与强化规则.html'
html = source.read_text()
html = html.replace('<title>ByeType设置界面对齐稿v8</title>', '<title>ByeType设置界面对齐稿v9</title>')
# 只改界面标签和编辑器名称，不改规则正文或交互逻辑。
for old, new in [
    ('语音转写', '语音识别'),
    ('转写设置', '语音识别设置'),
    ('转录提示词', '语音识别规则'),
    ('文本强化提示词', '文本强化规则'),
    ('转写模型', '语音识别模型'),
    ('转录规则', '识别规则'),
]:
    html = html.replace('>' + old + '<', '>' + new + '<')
    html = html.replace('aria-label="' + old + '"', 'aria-label="' + new + '"')
assert '>语音识别设置<' in html and '>语音识别规则<' in html and '>文本强化规则<' in html
assert not output.exists(), '不覆盖旧版对齐稿'
output.write_text(html)

check = (root / '02_work/check-v8.cjs').read_text()
check = check.replace(source.name, output.name)
check = check.replace('文本强化提示词', '文本强化规则').replace("includes('转录提示词')", "includes('语音识别规则')")
check += "\nassert(html.includes('>语音识别</div>'));\nassert(html.includes('>语音识别设置</button>'));\nassert(html.includes('>语音识别规则</button>'));\nassert(html.includes('>文本强化规则</button>'));\nconsole.log('通过：侧栏与页面名称统一，规则正文保持原样。');\n"
check_path = root / '02_work/check-v9.cjs'
assert not check_path.exists()
check_path.write_text(check)
print(output)
