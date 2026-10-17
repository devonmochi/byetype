from pathlib import Path
from datetime import datetime
from html import escape
import re

ROOT = Path('/Users/lishaojie/PycharmProjects/pythonProject1/日常工具/byetype')
OLD = ROOT / 'design/settings-rework/03_output/20261015-1200-设置界面展示稿v4-原版布局加独立优化开关.html'
css = re.search(r'<style>([\s\S]*?)</style>', OLD.read_text()).group(1)
css += '''
[hidden] { display: none !important; }
body { font-family: -apple-system, "PingFang SC", "Helvetica Neue", sans-serif; }
.content { min-width: 0; }
.frame { height: min(800px, calc(100dvh - 48px)); min-height: 560px; }
.setting-group-title { padding-left: 0; }
.toggle { background: var(--input-border); }
button:focus-visible, input:focus-visible, select:focus-visible, textarea:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
button:active { opacity: .8; }
.setting-row { gap: 12px; }
.actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.shortcut-actions { display: flex; align-items: center; gap: 10px; }
.mode-control { display: flex; align-items: center; gap: 12px; }
.mode-state { font-size: 12px; color: var(--text-secondary); }
.mode-label { font-weight: 500; }
.btn-link { color: var(--accent); background: none; border: none; cursor: pointer; font: inherit; font-size: 12px; padding: 0; }
.prompt-input { display: block; width: 100%; min-height: 240px; resize: vertical; padding: 12px; border: 1px solid var(--input-border); border-radius: 6px; background: var(--input-bg); color: var(--text-primary); font: 12px/1.7 ui-monospace, 'SF Mono', Menlo, monospace; }
.template-tools { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin: 20px 0 10px; }
.template-select { min-width: 160px; }
.status-line { min-height: 24px; color: var(--text-secondary); font-size: 12px; padding: 6px 0; }
.status-line.error { color: #c9352b; }
.key-field { display: flex; gap: 8px; flex: 1; align-items: center; }
.key-field input { flex: 1; min-width: 0; }
.kbd { font: 12px -apple-system, monospace; }
.section-title:first-child { margin-top: 0; }
.simple-list { background: var(--group-bg); border-radius: 10px; padding: 12px 16px; margin-bottom: 10px; }
.record-head { display: flex; gap: 16px; justify-content: space-between; align-items: center; font-size: 12px; color: var(--text-secondary); margin-bottom: 8px; }
.record-text { display: flex; align-items: center; gap: 12px; font-size: 13px; }
.record-text span { flex: 1; }
.stats { display: grid; grid-template-columns: repeat(4, minmax(0,1fr)); gap: 10px; margin: 16px 0; }
.stat { background: var(--group-bg); border-radius: 10px; padding: 14px; min-width: 0; }
.stat label { display: block; font-size: 11px; color: var(--text-secondary); margin-bottom: 8px; }
.stat b { font-size: 17px; font-variant-numeric: tabular-nums; }
.summary-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.summary-table td, .summary-table th { padding: 10px; text-align: right; border-bottom: 1px solid var(--border-color-light); }
.summary-table td:first-child, .summary-table th:first-child { text-align: left; }
.summary-table th { font-weight: 500; color: var(--text-secondary); }
dialog { background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border-color); border-radius: 12px; width: min(560px,90vw); max-height: 85dvh; overflow: auto; padding: 24px; margin: auto; }
dialog::backdrop { background: rgb(0 0 0 / .25); }
dialog h2 { font-size: 16px; margin-bottom: 16px; }
.form-field { display: block; font-size: 12px; margin: 12px 0; }
.form-field .input, .form-field .select { margin-top: 6px; width: 100%; }
.dialog-footer { display: flex; justify-content: flex-end; gap: 8px; margin-top: 20px; }
.empty { text-align: center; font-size: 13px; color: var(--text-secondary); padding: 30px; }
@media (max-width: 700px) { body { padding: 0; } .frame { height: 100dvh; min-height: 0; border-radius: 0; } .sidebar { width: 144px; } .content { padding: 18px; } .setting-row { flex-wrap: wrap; } .shortcut-actions { flex-wrap: wrap; } .stats { grid-template-columns: repeat(2,minmax(0,1fr)); } }
'''

def input_(val='', kind='text', width='100px', ident='', **attrs):
    extras = ''.join(f' {k}="{escape(str(v), quote=True)}"' for k,v in attrs.items())
    return f'<input class="input" type="{kind}" value="{escape(str(val), quote=True)}" style="width:{width}"' + (f' id="{ident}"' if ident else '') + extras + '>'

def select(options, ident='', width='260px'):
    return f'<select class="select" style="max-width:100%;width:{width}"' + (f' id="{ident}"' if ident else '') + '>' + ''.join(f'<option value="{escape(str(v), quote=True)}">{escape(str(label))}</option>' for v,label in options) + '</select>'

def toggle(label, on=False, ident=''):
    return f'<button type="button" class="toggle{" on" if on else ""}" role="switch" aria-checked="{str(on).lower()}" aria-label="{label}" onclick="toggle(this)"' + (f' id="{ident}"' if ident else '') + '><span class="toggle-knob"></span></button>'

def row(label, control):
    return f'<div class="setting-row"><div class="setting-row-main"><div class="setting-row-label">{label}</div></div>{control}</div>'

def group(title, body, gated=False):
    return f'<div' + (' data-opt-gated=""' if gated else '') + f'><div class="setting-group-title">{title}</div><div class="setting-group">{body}</div></div>'

def page(key, title, body, gated=False):
    return f'<section class="page" data-page="{key}"' + (' data-opt-gated=""' if gated else '') + f'><h2 class="content-title">{title}</h2>{body}</section>'

models = [
    ('gemini', 'Google Gemini', 'gemini-3.8-flash', True, True, True),
    ('dashscope', '阿里云百炼', 'qwen3.8-omni-flash', True, True, True),
    ('mimo', 'XiaoMi', 'mimo-v2.5', True, True, True),
    ('openrouter', 'OpenRouter', 'google/gemini-3.8-flash', True, True, True),
    ('openrouter-lite', 'OpenRouter', 'google/gemini-3.5-flash-lite', True, True, True),
    ('deepseek', 'DeepSeek', 'deepseek-flash', False, True, True),
]
audio = [(i,f'{p} - {m}') for i,p,m,a,t,v in models if a]
text = [('deepseek','DeepSeek - deepseek-flash')] + [(i,f'{p} - {m}') for i,p,m,a,t,v in models if i != 'deepseek']
vision = [(i,f'{p} - {m}') for i,p,m,a,t,v in models if v]

def shortcut(label, key, default_style, ident, voice=True):
    styles = [('voice-optimize','自动换行'),('voice-translate','翻译'),('voice-custom','自定义')] if voice else [('image-extract','文字识别'),('image-translate','翻译'),('image-custom','自定义')]
    styles = sorted(styles,key=lambda s: s[0] != default_style)
    return row(f'<span contenteditable="true" aria-label="{label}名称">{label}</span>', '<div class="shortcut-actions">' + ('<span class="actions" data-opt-gated="">' if voice else '<span class="actions">') + '<span class="mode-state">输出风格</span>' + select(styles, ident=ident, width='120px') + '</span>' + f'<button class="kbd" onclick="recordKey(this)" aria-label="修改{label}快捷键">{key}</button></div>')

mode_row = '''<div class="setting-row" id="optimization-row"><div class="setting-row-main"><div class="setting-row-label mode-label">文本优化</div></div><div class="mode-control"><button class="btn-link" id="edit-prompts" data-opt-gated="" onclick="go('voice-prompts')">编辑提示词</button><span class="mode-state" id="optimization-state">已开启</span><button class="toggle on" id="optimization-switch" type="button" role="switch" aria-label="文本优化" aria-checked="true" aria-controls="prompt-navigation voice-prompt-page" onclick="setOptimization(!optimizationEnabled)"><span class="toggle-knob"></span></button></div></div>'''
voice_rows = mode_row + shortcut('语音输入 1','F4','voice-optimize','shortcut-template') + shortcut('语音输入 2','未设置','voice-translate','shortcut2-template') + row('按住说话模式', toggle('按住说话模式')) + row('覆盖剪贴板', toggle('覆盖剪贴板',True))
appearance = '<div class="appearance-options" style="padding:12px 16px">' + ''.join(f'<button class="appearance-option{" active" if theme=="system" else ""}" onclick="setTheme(\'{theme}\',this)"><div class="appearance-preview" style="background:{bg};border:1px solid var(--input-border)"></div><div class="appearance-label">{label}</div></button>' for theme,label,bg in [('light','浅色','#fff'),('dark','深色','#1c1c1e'),('system','自动','linear-gradient(to right,#fff 50%,#1c1c1e 50%)')]) + '</div>'
network = row('本机转写接口',toggle('本机转写接口',ident='api-switch')) + row('本机接口端口',input_(8765,'number',min=1024,max=65535)) + row('接口状态','<span class="mode-state" id="api-state">未开启</span>') + row('curl示例','<button class="btn" onclick="showCurl()">查看示例</button>') + row('转写超时时间', input_(10,'number',min=1)) + row('文本优化超时时间', input_(10,'number',min=1)) + row('最大重试次数',input_(3,'number',min=0)) + row('最大并行任务数',input_(3,'number',min=1)) + row('HTTP代理',toggle('HTTP代理',True)) + row('代理地址',input_('','text','280px',placeholder='http://127.0.0.1:10809'))
general = page('general','通用设置', group('外观',appearance) + group('语音输入',voice_rows) + group('图像识别',shortcut('截图取词','F6','image-extract','extract-template',False) + shortcut('截图翻译','未设置','image-translate','extract2-template',False)) + group('其他',row('最大录音时长','<span class="actions">'+input_(180,'number','80px',min=10,max=600)+'<span class="mode-state">秒</span></span>') + row('开机自启',toggle('开机自启'))) + group('麦克风',row('输入设备','<span class="actions">'+select([('default','系统默认'),('builtin','MacBook Pro麦克风')],width='200px') + '<button class="btn" onclick="toast(\'已刷新\')">刷新</button></span>')) + group('网络与性能',network))

transcribe = page('transcribe','转写设置', group('转写模型',row('转写模型',select(audio)) + row('启用思考',toggle('转写思考'))) + group('文本优化模型',row('处理模型',select(text)) + row('启用思考',toggle('文本优化思考',True)) + row('Reasoning Effort',select([(x,x) for x in ['high','low','max']],width='120px')), True) + group('其他',row('规则增强',toggle('规则增强')), True))

def prompt_text(filename):
    return (ROOT / 'src-tauri/prompts' / filename).read_text()

prompt_parts = []
for idx,(key,label,file) in enumerate([('agent','角色定义','agent.md'),('rules','转录规则','rules.md'),('vocabulary','专有词汇','vocabulary.md')]):
    prompt_parts.append(f'<div data-prompt-doc="{key}"'+(' hidden' if idx else '')+'><div class="prompt-path-bar"><span class="path-text">'+file+'</span><button class="btn" onclick="choosePromptFile(\''+key+'\')">选择文件</button><button class="btn" onclick="resetPrompt(\''+key+'\')">重置为内置</button></div><textarea class="prompt-input" id="prompt-'+key+'" aria-label="'+label+'">'+escape(prompt_text(file))+'</textarea></div>')
prompts_page = page('voice-prompts','转写提示词','<div class="prompt-tabs">'+''.join(f'<button class="prompt-tab{" active" if idx==0 else ""}" onclick="promptTab(\'{key}\',this)">{label}</button>' for idx,(key,label) in enumerate([('agent','角色定义'),('rules','转录规则'),('vocabulary','专有词汇')]))+'</div>' + ''.join(prompt_parts) + '<h2 class="content-title" style="margin-top:24px">文本优化提示词</h2><div class="template-tools"><select class="select template-select" id="voice-template-list" onchange="showTemplate(\'voice\')"></select><div class="actions"><button class="btn" onclick="renameTemplate(\'voice\')">重命名</button><button class="btn" onclick="addTemplate(\'voice\')">添加模板</button><button class="btn" id="voice-delete" onclick="deleteTemplate(\'voice\')">删除</button></div></div><div class="prompt-path-bar"><span class="path-text" id="voice-template-path"></span><button class="btn" onclick="chooseTemplateFile(\'voice\')">选择文件</button><button class="btn" id="voice-reset" onclick="resetTemplate(\'voice\')">重置为内置</button></div><textarea class="prompt-input" id="voice-template-editor" aria-label="文本优化提示词"></textarea>',True).replace('data-page="voice-prompts"','data-page="voice-prompts" id="voice-prompt-page"')
extract = page('extract','图像识别设置',group('模型',row('图像识别模型',select(vision))))
extract_prompts = page('extract-prompts','图像识别提示词','<div class="template-tools"><select class="select template-select" id="image-template-list" onchange="showTemplate(\'image\')"></select><div class="actions"><button class="btn" onclick="renameTemplate(\'image\')">重命名</button><button class="btn" onclick="addTemplate(\'image\')">添加模板</button><button class="btn" id="image-delete" onclick="deleteTemplate(\'image\')">删除</button></div></div><div class="prompt-path-bar"><span class="path-text" id="image-template-path"></span><button class="btn" onclick="chooseTemplateFile(\'image\')">选择文件</button><button class="btn" id="image-reset" onclick="resetTemplate(\'image\')">重置为内置</button></div><textarea class="prompt-input" id="image-template-editor" aria-label="图像识别提示词"></textarea>')
learning = page('voice-learning','自动学习',group('模型',row('学习模型',select(text)) + row('启用思考',toggle('学习思考',True)) + row('Reasoning Effort',select([(x,x) for x in ['high','low','max']],width='120px'))) + '<h3 class="section-title">学习文档</h3><div class="prompt-tabs"><button class="prompt-tab active" onclick="learningTab(\'prompt\',this)">学习提示词</button><button class="prompt-tab" onclick="learningTab(\'result\',this)">学习结果</button></div><textarea class="prompt-input" id="learning-prompt" aria-label="学习提示词">'+escape(prompt_text('voice-learning-prompt.md'))+'</textarea><textarea class="prompt-input" id="learning-result" aria-label="学习结果" hidden></textarea>')
provider_cards = ''
for provider in dict.fromkeys(m[1] for m in models):
    provider_id = next(m[0] for m in models if m[1]==provider)
    provider_cards += f'<div class="model-card"><div class="model-card-header"><span class="model-card-title">{provider}</span></div><div class="model-card-row"><label for="key-{provider_id}">API Key</label><div class="key-field"><input class="input" type="password" id="key-{provider_id}" placeholder="填写密钥" autocomplete="off"><button class="btn" onclick="togglePwd(\'key-{provider_id}\',this)">显示</button></div></div><div class="provider-model-list">'
    for ident,p,m,a,t,v in models:
        if p != provider: continue
        caps = ''.join(f'<span class="cap-tag cap-{c}">{l}</span>' for c,l,yes in [('audio','音频',a),('vision','图像',v),('text','文本',t)] if yes)
        provider_cards += f'<div class="provider-model-item"><span class="provider-model-name">{m}</span><span class="model-caps">{caps}</span><button class="btn" onclick="testModel(\'{provider_id}\',this)">测试</button></div>'
    provider_cards += '</div></div>'
model_page = page('models','模型管理','<div class="actions" style="justify-content:flex-end;margin-bottom:12px"><button class="btn" onclick="testAllModels(this)">测试全部连通性</button></div><div class="models-section-title">预置模型</div>'+provider_cards+'<div class="models-section-title">自定义模型</div><div id="custom-models"></div><button class="add-model-btn" onclick="document.getElementById(\'custom-model-dialog\').showModal()">+ 添加自定义模型</button>')

history = page('history','历史记录','<div class="simple-list"><div class="record-head"><span>10:24:08</span><span>音频 → 转写 → 优化</span><button class="btn" onclick="toast(\'已提交重试\')">重试</button></div><div class="record-text"><span>明天上午10点开会。</span><button class="btn" onclick="copyText(\'明天上午10点开会。\')">复制</button></div></div><div class="simple-list"><div class="record-head"><span>10:18:55</span><span>截图 → 识别失败</span><button class="btn" onclick="toast(\'已提交重试\')">重试</button></div><div class="status-line error">调用超时</div></div>')
usage = page('usage','用量统计','<div class="prompt-tabs"><button class="prompt-tab active" onclick="usageTab(\'token\',this)">Token消耗</button><button class="prompt-tab" onclick="usageTab(\'timing\',this)">处理耗时</button></div><div class="actions" style="margin:14px 0"><select class="select"><option>今天</option><option>近7天</option><option>近30天</option><option>全部</option></select><select class="select" id="usage-scene"><option>全部场景</option><option>转写</option><option>识别</option><option>优化</option><option>学习</option></select></div><div id="usage-token"><div class="stats"><div class="stat"><label>总Token</label><b>48.2万</b></div><div class="stat"><label>API调用</label><b>28次</b></div><div class="stat"><label>最常用模型</label><b style="font-size:12px">gemini-3.8-flash</b></div><div class="stat"><label>消耗趋势</label><b>+12%</b></div></div><table class="summary-table"><thead><tr><th>模型</th><th>输入Token</th><th>输出Token</th><th>调用次数</th></tr></thead><tbody><tr><td>gemini-3.8-flash</td><td>30.1万</td><td>2.4万</td><td>18</td></tr><tr><td>deepseek-flash</td><td>11.4万</td><td>4.3万</td><td>10</td></tr></tbody></table></div><div id="usage-timing" hidden><div class="stats"><div class="stat"><label>平均处理耗时</label><b>3.8秒</b></div><div class="stat"><label>音频转写平均</label><b>2.9秒</b></div><div class="stat"><label>文本优化平均</label><b>0.8秒</b></div><div class="stat"><label>录音次数</label><b>22次</b></div></div></div><div class="actions" style="justify-content:flex-end;margin-top:16px"><button class="btn" onclick="confirmAction(\'清空统计数据\',\'确认清空全部统计数据？\',\'已清空统计数据\')">清空统计数据</button></div>')
backup_fields = ''.join(row(label,input_(value,kind,'260px',ident)) for label,value,kind,ident in [('Endpoint','','text','s3-endpoint'),('Region','','text','s3-region'),('Bucket','','text','s3-bucket'),('Access Key','','password','s3-access'),('Secret Key','','password','s3-secret'),('路径前缀','byetype/backups','text','s3-prefix')])
backup = page('backup','备份与恢复',group('S3兼容存储配置',backup_fields + row('连接测试','<button class="btn" onclick="testS3(this)">测试连接</button>')) + group('S3备份',row('立即备份到S3','<button class="btn" onclick="testS3(this)">备份到S3</button>') + row('从S3恢复','<button class="btn" onclick="document.getElementById(\'backup-list\').hidden=false">刷新列表</button>')+'<div class="empty" id="backup-list" hidden>暂无备份</div>') + group('本地备份',row('备份到本地','<button class="btn" onclick="toast(\'备份已导出\')">备份到本地</button>') + row('从本地恢复','<button class="btn" onclick="confirmAction(\'从本地恢复\',\'当前配置将被覆盖，恢复后需要重启应用。\',\'恢复成功，请重启应用\')">从本地恢复</button>')))
about = page('about','关于','<div class="about-header"><div class="about-app-icon"><svg width="36" height="38" viewBox="0 0 92 94"><rect x="1" y="30" width="16" height="34" rx="8" fill="white"/><rect x="19.5" y="12" width="16" height="70" rx="8" fill="white"/><rect x="38" y="2" width="16" height="90" rx="8" fill="white"/><rect x="56.5" y="12" width="16" height="70" rx="8" fill="white"/><rect x="75" y="30" width="16" height="34" rx="8" fill="white"/></svg></div><div class="about-app-name">ByeType</div><div class="about-app-version">版本1.22.4</div><button class="btn" onclick="toast(\'已是最新版本\')">检查更新</button></div>')

nav_items = [('general','通用设置'),('models','模型管理'),(None,'语音转写'),('transcribe','转写设置'),('voice-prompts','转写提示词'),(None,'图像识别'),('extract','图像识别设置'),('extract-prompts','图像识别提示词'),(None,'智能学习'),('voice-learning','自动学习'),(None,''),('history','历史记录'),('usage','用量统计'),('backup','备份与恢复'),('about','关于')]
nav=''
for key,label in nav_items:
    if key is None:
        nav += f'<div class="sidebar-group">{label}</div>' if label else '<div class="sidebar-divider"></div>'
    else:
        gate = ' id="prompt-navigation" data-opt-gated=""' if key=='voice-prompts' else ''
        nav += f'<button class="sidebar-item" data-page="{key}"{gate} onclick="go(\'{key}\')">{label}</button>'

import json
templates = {
    'voice': [dict(id=i,name=n,file=f,text=prompt_text(f) if f else '',builtin=True) for i,n,f in [('voice-optimize','自动换行','text-optimize.md'),('voice-translate','翻译','voice-translate.md'),('voice-custom','自定义','')]],
    'image': [dict(id=i,name=n,file=f,text=prompt_text(f) if f else '',builtin=True) for i,n,f in [('image-extract','文字识别','text-extract.md'),('image-translate','翻译','image-translate.md'),('image-custom','自定义','')]]
}
script = 'const templates = ' + json.dumps(templates,ensure_ascii=False).replace('</','<\\/') + ';\n'
script += '''
let optimizationEnabled = true;
let activePage = 'general';
const defaults = {};
const originalTemplates = JSON.parse(JSON.stringify(templates));
let templateCounter = 0;
let confirmCallback = () => {};
function go(id) {
  if (id === 'voice-prompts' && !optimizationEnabled) id = 'general';
  activePage = id;
  document.querySelectorAll('.sidebar-item').forEach(b => b.classList.toggle('active', b.dataset.page === id));
  document.querySelectorAll('.page').forEach(p => p.classList.toggle('active', p.dataset.page === id));
  document.getElementById('content').scrollTop = 0;
}
function setOptimization(on) {
  optimizationEnabled = !!on;
  const button = document.getElementById('optimization-switch');
  button.classList.toggle('on', optimizationEnabled);
  button.setAttribute('aria-checked', String(optimizationEnabled));
  document.getElementById('optimization-state').textContent = optimizationEnabled ? '已开启' : '已关闭';
  document.querySelectorAll('[data-opt-gated]').forEach(el => { el.hidden = !optimizationEnabled; });
  if (!optimizationEnabled && activePage === 'voice-prompts') go('general');
  saved();
}
function toggle(btn) {
  const on = btn.getAttribute('aria-checked') !== 'true';
  btn.classList.toggle('on',on); btn.setAttribute('aria-checked',String(on));
  if (btn.id === 'api-switch') document.getElementById('api-state').textContent = on ? '运行中：127.0.0.1:8765' : '未开启';
  saved();
}
function setTheme(mode,btn) {
  document.querySelectorAll('.appearance-option').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
  document.documentElement.dataset.theme = mode === 'dark' || (mode === 'system' && matchMedia('(prefers-color-scheme: dark)').matches) ? 'dark' : 'light';
  saved();
}
let savedTimer, toastTimer;
function saved() { const el=document.getElementById('savedToast'); el.textContent='✓ 已保存'; clearTimeout(savedTimer); savedTimer=setTimeout(()=>{el.textContent='';},1500); }
function toast(msg) { const el=document.getElementById('toast'); el.textContent=msg; el.classList.add('show'); clearTimeout(toastTimer); toastTimer=setTimeout(()=>el.classList.remove('show'),1800); }
function promptTab(key,btn) {
  document.querySelectorAll('[data-prompt-doc]').forEach(el=>{el.hidden=el.dataset.promptDoc!==key;});
  btn.parentElement.querySelectorAll('.prompt-tab').forEach(b=>b.classList.toggle('active',b===btn));
}
function selectedTemplate(kind) { return templates[kind].find(t=>t.id===document.getElementById(kind+'-template-list').value); }
function refreshTemplates(kind, selected) {
  const list=document.getElementById(kind+'-template-list'); list.replaceChildren();
  templates[kind].forEach(t=>{ const o=document.createElement('option'); o.value=t.id; o.textContent=t.name; list.append(o); });
  if (selected) list.value=selected;
  showTemplate(kind);
  const ids = kind==='voice' ? ['shortcut-template','shortcut2-template'] : ['extract-template','extract2-template'];
  ids.forEach(id=>{ const select=document.getElementById(id); const old=select.value; select.replaceChildren(); templates[kind].forEach(t=>{const o=document.createElement('option'); o.value=t.id; o.textContent=t.name; select.append(o);}); if(templates[kind].some(t=>t.id===old)) select.value=old; });
}
function showTemplate(kind) {
  const t=selectedTemplate(kind); if(!t)return;
  document.getElementById(kind+'-template-editor').value=t.text;
  document.getElementById(kind+'-template-path').textContent=t.file || '自定义提示词';
  document.getElementById(kind+'-reset').hidden=!t.file || !t.builtin;
  document.getElementById(kind+'-delete').hidden=t.builtin;
}
function addTemplate(kind) {
  const t={id:kind+'-user-'+(++templateCounter),name:'新模板',file:'',text:'',builtin:false};
  templates[kind].push(t); refreshTemplates(kind,t.id); saved();
}
function renameTemplate(kind) {
  const t=selectedTemplate(kind); openNameDialog('重命名模板',t.name,value=>{t.name=value;refreshTemplates(kind,t.id);saved();});
}
function deleteTemplate(kind) { const t=selectedTemplate(kind); if(t.builtin)return; templates[kind]=templates[kind].filter(x=>x.id!==t.id);refreshTemplates(kind);saved(); }
function resetTemplate(kind) {
  const t=selectedTemplate(kind); const original=originalTemplates[kind].find(x=>x.id===t.id); if(!original||!original.file)return;
  confirmAction('重置为内置','当前修改将被覆盖。','已重置',()=>{t.text=original.text;showTemplate(kind);saved();});
}
function resetPrompt(key) { confirmAction('重置为内置','当前修改将被覆盖。','已重置',()=>{document.getElementById('prompt-'+key).value=defaults[key];saved();}); }
let fileHandler = null;
function readFileInto(callback) { fileHandler=callback;document.getElementById('prompt-file').click(); }
function choosePromptFile(key) { readFileInto(text=>{document.getElementById('prompt-'+key).value=text;saved();}); }
function chooseTemplateFile(kind) { const t=selectedTemplate(kind);readFileInto((text,name)=>{t.text=text;t.file=name;showTemplate(kind);saved();}); }
function learningTab(key,btn) { document.getElementById('learning-prompt').hidden=key!=='prompt';document.getElementById('learning-result').hidden=key!=='result';btn.parentElement.querySelectorAll('.prompt-tab').forEach(b=>b.classList.toggle('active',b===btn)); }
function usageTab(key,btn) { document.getElementById('usage-token').hidden=key!=='token';document.getElementById('usage-timing').hidden=key!=='timing';document.getElementById('usage-scene').hidden=key!=='token';btn.parentElement.querySelectorAll('.prompt-tab').forEach(b=>b.classList.toggle('active',b===btn)); }
function togglePwd(id,btn) { const input=document.getElementById(id);input.type=input.type==='password'?'text':'password';btn.textContent=input.type==='password'?'显示':'隐藏'; }
function testModel(provider,btn) { const old=btn.textContent;btn.disabled=true;btn.textContent='测试中…';setTimeout(()=>{btn.disabled=false;btn.textContent=old;toast(document.getElementById('key-'+provider).value.trim()?'连接成功':'密钥未填写');},450); }
function testAllModels(btn) { btn.disabled=true;btn.textContent='测试中…';setTimeout(()=>{btn.disabled=false;btn.textContent='测试全部连通性';toast('请填写密钥');},450); }
function testS3(btn) { if(!document.getElementById('s3-bucket').value.trim()){toast('请填写Bucket');return;} toast('连接成功'); }
function showCurl() { document.getElementById('curl-dialog').showModal(); }
async function copyText(text) { try{await navigator.clipboard.writeText(text);toast('已复制');}catch{toast('复制失败');} }
function confirmAction(title,body,msg,action) { const d=document.getElementById('confirm-dialog');document.getElementById('confirm-title').textContent=title;document.getElementById('confirm-body').textContent=body;confirmCallback=()=>{if(action)action();toast(msg);};d.showModal(); }
let nameCallback = () => {};
function openNameDialog(title,value,callback) { document.getElementById('name-title').textContent=title;document.getElementById('name-input').value=value;nameCallback=callback;document.getElementById('name-dialog').showModal(); }
function saveName() { const value=document.getElementById('name-input').value.trim();if(!value)return;nameCallback(value);document.getElementById('name-dialog').close(); }
let keyHandler = null;
function recordKey(btn) {
  if(keyHandler)window.removeEventListener('keydown',keyHandler,true);
  const old=btn.textContent;btn.textContent='按下快捷键';
  keyHandler=e=>{
    e.preventDefault();if(['Meta','Control','Alt','Shift'].includes(e.key))return;
    window.removeEventListener('keydown',keyHandler,true);keyHandler=null;
    if(e.key==='Escape'){btn.textContent=old;return;}
    const parts=[];if(e.metaKey)parts.push('⌘');if(e.ctrlKey)parts.push('Ctrl');if(e.altKey)parts.push('⌥');if(e.shiftKey)parts.push('⇧');parts.push(e.key===' '?'Space':e.key.toUpperCase());
    const value=parts.join('+');
    const conflict=[...document.querySelectorAll('.kbd')].find(x=>x!==btn && x.textContent===value);
    if(conflict){btn.textContent=old;toast('快捷键已被使用');return;}
    btn.textContent=value;saved();
  };
  window.addEventListener('keydown',keyHandler,true);
}
function saveCustomModel(event) {
  event.preventDefault();const form=event.currentTarget;
  try{const value=JSON.parse(form.elements.kwargs.value);if(!value||Array.isArray(value)||typeof value!=='object')throw new Error();}catch{document.getElementById('custom-error').textContent='附加参数必须是JSON对象';return;}
  const el=document.createElement('div');el.className='model-card';el.textContent=form.elements.provider.value+' - '+form.elements.model.value;document.getElementById('custom-models').append(el);document.getElementById('custom-model-dialog').close();saved();
}
['agent','rules','vocabulary'].forEach(key=>{defaults[key]=document.getElementById('prompt-'+key).value;});
['voice','image'].forEach(kind=>{refreshTemplates(kind);document.getElementById(kind+'-template-editor').addEventListener('input',e=>{selectedTemplate(kind).text=e.target.value;saved();});});
document.getElementById('prompt-file').addEventListener('change',async e=>{const file=e.target.files[0];if(file&&fileHandler)fileHandler(await file.text(),file.name);e.target.value='';});
document.querySelectorAll('input,select,textarea,[contenteditable]').forEach(el=>el.addEventListener('change',saved));
if(matchMedia('(prefers-color-scheme: dark)').matches)document.documentElement.dataset.theme='dark';
go('general');
'''
dialogs = '''
<input type="file" id="prompt-file" accept=".md,.txt" hidden>
<dialog id="confirm-dialog"><h2 id="confirm-title"></h2><p id="confirm-body" style="font-size:13px"></p><div class="dialog-footer"><button class="btn" onclick="document.getElementById('confirm-dialog').close()">取消</button><button class="btn" onclick="confirmCallback();document.getElementById('confirm-dialog').close()">确认</button></div></dialog>
<dialog id="name-dialog"><h2 id="name-title"></h2><label class="form-field">名称<input class="input" id="name-input"></label><div class="dialog-footer"><button class="btn" onclick="document.getElementById('name-dialog').close()">取消</button><button class="btn" onclick="saveName()">保存</button></div></dialog>
<dialog id="curl-dialog"><h2>curl示例</h2><textarea class="prompt-input" id="curl-example" readonly>curl -fsS -X POST --data-binary @recording.m4a -H 'Content-Type: audio/mp4' 'http://127.0.0.1:8765/transcribe'</textarea><div class="dialog-footer"><button class="btn" onclick="copyText(document.getElementById('curl-example').value)">复制</button><button class="btn" onclick="document.getElementById('curl-dialog').close()">关闭</button></div></dialog>
<dialog id="custom-model-dialog"><form onsubmit="saveCustomModel(event)"><h2>添加自定义模型</h2><label class="form-field">提供商<input name="provider" class="input" required></label><label class="form-field">模型ID<input name="model" class="input" required></label><label class="form-field">协议<select name="protocol" class="select"><option>OpenAI兼容</option><option>Gemini</option><option>OpenRouter</option></select></label><label class="form-field">Base URL<input name="url" class="input" type="url" required></label><label class="form-field">API Key<input name="key" class="input" type="password"></label><div class="actions" style="font-size:12px"><label><input type="checkbox" checked>音频</label><label><input type="checkbox" checked>图像</label><label><input type="checkbox" checked>文本</label></div><label class="form-field">音频请求格式<select class="select"><option>input_audio</option><option>audio_url</option></select></label><label class="form-field">chat_template_kwargs<textarea name="kwargs" class="prompt-input" style="min-height:65px">{}</textarea></label><div id="custom-error" class="status-line error"></div><div class="dialog-footer"><button type="button" class="btn" onclick="document.getElementById('custom-model-dialog').close()">取消</button><button class="btn" type="submit">保存</button></div></form></dialog>
'''
html = '<!DOCTYPE html>\n<html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>ByeType设置界面v5</title><style>'+css+'</style></head><body><div class="frame"><nav class="sidebar" id="sidebar">'+nav+'</nav><main class="content" id="content"><span class="saved-toast" id="savedToast" aria-live="polite"></span>'+general+transcribe+prompts_page+extract+extract_prompts+learning+model_page+history+usage+backup+about+'</main></div><div class="toast" id="toast" role="status"></div>'+dialogs+'<script>'+script+'</script></body></html>'
out = ROOT / 'design/settings-rework/03_output/20261015-1213-设置界面v5-独立开关联动提示词.html'
assert not out.exists(), '不覆盖旧版'
out.write_text(html)
print(out)
