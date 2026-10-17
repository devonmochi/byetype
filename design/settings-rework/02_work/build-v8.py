from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
source = ROOT / '03_output/20261015-1242-设置界面v7-文本强化.html'
output = ROOT / '03_output/20261015-1251-设置界面v8-拆分转录与强化提示词.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:120]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面对齐稿v7</title>', '<title>ByeType设置界面对齐稿v8</title>')
# 只调整首页顺序，输入1和输入2都排在开关上方。
start = html.index('<div class="setting-row" id="optimization-row">')
end = html.index('<div class="setting-row" id="secondary-voice-row"', start)
mode_row = html[start:end]
assert mode_row.count('id="optimization-switch"') == 1
html = html[:start] + html[end:]
next_row = '<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label">按住说话模式</div>'
assert html.count(next_row) == 1
html = html.replace(next_row, mode_row + next_row, 1)

# 两个独立侧栏入口和两个独立页面，不再共用一个长页面。
old_nav = '<button class="sidebar-item" data-page="voice-prompts" id="prompt-navigation" data-opt-gated="" onclick="go(\'voice-prompts\')"><span>转写提示词</span><span class="optimization-marker" aria-label="文本强化相关">强化</span></button>'
new_nav = '<button class="sidebar-item" data-page="voice-prompts" id="prompt-navigation" data-opt-gated="" onclick="go(\'voice-prompts\')">转录提示词</button><button class="sidebar-item" data-page="strengthening-prompts" id="strengthening-navigation" data-opt-gated="" onclick="go(\'strengthening-prompts\')">文本强化提示词</button>'
replace(old_nav, new_nav)
replace('<h2 class="content-title optimization-heading">转写提示词<span class="optimization-marker">强化</span></h2>', '<h2 class="content-title">转录提示词</h2>')
replace('<h2 class="content-title optimization-heading" style="margin-top:24px">文本强化提示词<span class="optimization-marker">强化</span></h2>', '</section><section class="page" data-page="strengthening-prompts" id="strengthening-prompt-page" data-opt-gated=""><h2 class="content-title optimization-heading">文本强化提示词</h2>')
replace('#prompt-navigation { border-left: 2px solid var(--accent); padding-left: 14px; }', '#strengthening-navigation { border-left: 2px solid var(--accent); padding-left: 14px; color: var(--accent); }')
replace("if (id === 'voice-prompts' && !optimizationEnabled) id = 'general';", "if (['voice-prompts', 'strengthening-prompts'].includes(id) && !optimizationEnabled) id = 'general';")
replace("if (!optimizationEnabled && activePage === 'voice-prompts') go('general');", "if (!optimizationEnabled && ['voice-prompts', 'strengthening-prompts'].includes(activePage)) go('general');")
replace('aria-controls="prompt-navigation voice-prompt-page"', 'aria-controls="prompt-navigation voice-prompt-page strengthening-navigation strengthening-prompt-page secondary-voice-row"')
assert html.index('id="primary-voice-name"') < html.index('id="secondary-voice-row"') < html.index('id="optimization-switch"')
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

check = (ROOT / '02_work/check-v7.cjs').read_text()
check = check.replace('20261015-1242-设置界面v7-文本强化.html', output.name)
check = check.replace("assert(home.indexOf('id=\"optimization-switch\"') < home.indexOf('id=\"secondary-voice-row\"'));", "assert(home.indexOf('id=\"secondary-voice-row\"') < home.indexOf('id=\"optimization-switch\"'));")
check = check.replace("assert(html.includes('class=\"optimization-marker\">强化</span>'));", "assert(html.includes('id=\"strengthening-navigation\"'));")
check = check.replace("assert(html.includes('class=\"optimization-marker\"'));", "assert(html.includes('class=\"content-title optimization-heading\">文本强化提示词'));")
check = check.replace("assert(html.includes('#prompt-navigation { border-left: 2px solid var(--accent)'));", "assert(html.includes('#strengthening-navigation { border-left: 2px solid var(--accent)'));")
check = check.replace("assert.equal(ids.get('secondary-voice-row').hidden,false);\nconst styles", """const transcriptionPage = html.match(/<section class=\"page\" data-page=\"voice-prompts\"[^>]*>([\\s\\S]*?)<\\/section>/)[1];
const strengtheningPage = html.match(/<section class=\"page\" data-page=\"strengthening-prompts\"[^>]*>([\\s\\S]*?)<\\/section>/)[1];
assert(transcriptionPage.includes('转录提示词'));
assert(transcriptionPage.includes('角色定义') && transcriptionPage.includes('转录规则') && transcriptionPage.includes('专有词汇'));
assert(!transcriptionPage.includes('voice-template-editor'));
assert(strengtheningPage.includes('文本强化提示词') && strengtheningPage.includes('voice-template-editor'));
assert(!strengtheningPage.includes('prompt-agent') && !strengtheningPage.includes('prompt-rules'));
assert.equal(ids.get('secondary-voice-row').hidden,false);
const styles""")
check = check.replace("evaluate(\"go('voice-prompts')\");\nassert.equal(evaluate('activePage'),'general');", "evaluate(\"go('voice-prompts')\");\nassert.equal(evaluate('activePage'),'general');\nevaluate(\"go('strengthening-prompts')\");\nassert.equal(evaluate('activePage'),'general');")
check = check.replace("evaluate(\"go('voice-prompts')\");\nassert.equal(evaluate('activePage'),'voice-prompts');\nevaluate('setOptimization(false)');", "evaluate(\"go('voice-prompts')\");\nassert.equal(evaluate('activePage'),'voice-prompts');\nevaluate(\"go('strengthening-prompts')\");\nassert.equal(evaluate('activePage'),'strengthening-prompts');\nevaluate('setOptimization(false)');")
check = check.replace('通过：语音输入在前、文本强化开关在后、公式文案一致、强化入口标记、', '通过：两个语音入口均在开关上方、转录与强化提示词独立页面、强化入口配色、公式文案一致、')
check = check.replace('11个页面导航', '12个页面导航')
new_check = ROOT / '02_work/check-v8.cjs'
assert not new_check.exists(), '不覆盖检查脚本'
new_check.write_text(check)
print(output)
