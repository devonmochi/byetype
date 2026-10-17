from pathlib import Path

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1307-设置界面v10-强化输入缩进.html'
output = root / '03_output/20261015-1335-设置界面v11-识别规则始终保留.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:100]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面对齐稿v10</title>', '<title>ByeType设置界面对齐稿v11</title>')
replace('id="prompt-navigation" data-opt-gated=""', 'id="prompt-navigation"')
replace('data-page="voice-prompts" id="voice-prompt-page" data-opt-gated=""', 'data-page="voice-prompts" id="voice-prompt-page"')
replace("if (['voice-prompts', 'strengthening-prompts'].includes(id) && !optimizationEnabled) id = 'general';", "if (id === 'strengthening-prompts' && !optimizationEnabled) id = 'general';")
replace("if (!optimizationEnabled && ['voice-prompts', 'strengthening-prompts'].includes(activePage)) go('general');", "if (!optimizationEnabled && activePage === 'strengthening-prompts') go('general');")
replace('aria-controls="prompt-navigation voice-prompt-page strengthening-navigation strengthening-prompt-page secondary-voice-row"', 'aria-controls="strengthening-navigation strengthening-prompt-page secondary-voice-row strengthened-voice-inputs"')
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

check = (root / '02_work/check-v10.cjs').read_text().replace(source.name, output.name)
anchor = "for(const e of document.querySelectorAll('[data-opt-gated]'))assert.equal(e.hidden,true);"
assert check.count(anchor) == 1
check = check.replace(anchor, anchor + "\nassert.equal(get('prompt-navigation').hidden,false);\nassert.equal(get('voice-prompt-page').hidden,false);\nevaluate(\"go('voice-prompts')\");assert.equal(evaluate('activePage'),'voice-prompts');\nevaluate('setOptimization(false)');assert.equal(evaluate('activePage'),'voice-prompts');\nassert.equal(get('voice-prompt-page').classList.contains('active'),true);\nassert.equal(get('strengthening-navigation').hidden,true);\nassert.equal(get('strengthening-prompt-page').hidden,true);")
check = check.replace('规则页面联动和导航；', '语音识别规则始终可见且可访问，仅强化规则随开关隐藏；')
check_path = root / '02_work/check-v11.cjs'
assert not check_path.exists()
check_path.write_text(check)
print(output)
