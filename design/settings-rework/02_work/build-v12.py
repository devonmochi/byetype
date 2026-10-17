from pathlib import Path
import re

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1335-设置界面v11-识别规则始终保留.html'
output = root / '03_output/20261015-1349-设置界面v12-双向规则增强.html'
html = source.read_text()
assert html.count('<title>ByeType设置界面对齐稿v11</title>') == 1
html = html.replace('<title>ByeType设置界面对齐稿v11</title>', '<title>ByeType设置界面对齐稿v12</title>')
match = re.search(r'<section class="page" data-page="transcribe">[\s\S]*?</section>', html)
assert match
old_page = match.group()
page = old_page

old_rule = '<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label">规则增强</div></div><button type="button" class="toggle" role="switch" aria-checked="false" aria-label="规则增强" onclick="toggle(this)"><span class="toggle-knob"></span></button></div>'
old_group = '<div data-opt-gated=""><div class="setting-group-title">其他</div><div class="setting-group">' + old_rule + '</div></div>'
assert page.count(old_group) == 1
page = page.replace(old_group, '')

def enhancement(ident, direction, gated=False):
    return '<div class="setting-row"' + (' data-opt-gated=""' if gated else '') + '><div class="setting-row-main"><div class="setting-row-label">规则增强<span class="rule-direction">' + direction + '</span></div></div><button type="button" class="toggle" id="' + ident + '" role="switch" aria-checked="false" aria-label="规则增强：' + direction + '" onclick="toggle(this)"><span class="toggle-knob"></span></button></div>'

voice_switch = '<button type="button" class="toggle" role="switch" aria-checked="false" aria-label="转写思考" onclick="toggle(this)"><span class="toggle-knob"></span></button></div>'
assert page.count(voice_switch) == 1
page = page.replace(voice_switch, voice_switch + enhancement('inject-text-into-voice', '将文本规则注入语音规则'))
ending = '</div></div></section>'
assert page.endswith(ending)
page = page[:-len(ending)] + enhancement('inject-voice-into-text', '将语音规则注入文本规则') + ending
html = html[:match.start()] + page + html[match.end():]
heading = '<h2 class="content-title optimization-heading">文本强化规则</h2>'
assert html.count(heading) == 1
html = html.replace(heading, heading + '<p class="rules-intro">按输出风格处理识别文本，例如分段、翻译。</p>')
assert html.count('</style>') == 1
html = html.replace('</style>', '\n.rule-direction { margin-left: 10px; font-size: 11px; font-weight: 400; color: var(--text-secondary); }\n.rules-intro { margin: -10px 0 16px; font-size: 11px; color: var(--text-secondary); line-height: 1.6; }\n</style>')
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

check = (root / '02_work/check-v11.cjs').read_text().replace(source.name, output.name)
check += '''
const voiceEnhancement=get('inject-text-into-voice');
const textEnhancement=get('inject-voice-into-text');
assert.equal(voiceEnhancement.getAttribute('aria-checked'),'false');
assert.equal(textEnhancement.getAttribute('aria-checked'),'false');
assert.equal(voiceEnhancement.getAttribute('aria-label'),'规则增强：将文本规则注入语音规则');
assert.equal(textEnhancement.getAttribute('aria-label'),'规则增强：将语音规则注入文本规则');
evaluate("toggle(document.getElementById('inject-text-into-voice'))");
assert.equal(voiceEnhancement.getAttribute('aria-checked'),'true');
assert.equal(textEnhancement.getAttribute('aria-checked'),'false');
evaluate("toggle(document.getElementById('inject-voice-into-text'))");
assert.equal(textEnhancement.getAttribute('aria-checked'),'true');
assert.equal(voiceEnhancement.getAttribute('aria-checked'),'true');
evaluate('setOptimization(false)');
assert.equal(voiceEnhancement.parentElement.hidden,false);
assert.equal(get('prompt-navigation').hidden,false);
evaluate('setOptimization(true)');
assert.equal(voiceEnhancement.getAttribute('aria-checked'),'true');
assert.equal(textEnhancement.getAttribute('aria-checked'),'true');
assert.equal(voiceEnhancement.parentElement.hidden,false);
assert(html.includes('按输出风格处理识别文本，例如分段、翻译。'));
console.log('通过：两处规则增强使用指定方向文案、开关独立、状态保留、文本强化规则顶部小字。');
'''
check_path = root / '02_work/check-v12.cjs'
assert not check_path.exists()
check_path.write_text(check)
print(output)
