const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const path = require('node:path');
const output = path.resolve(__dirname, '../03_output/20261015-1300-设置界面v9-语音识别与强化规则.html');
const html = fs.readFileSync(output, 'utf8');
const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
new vm.Script(script);
const tags = [...html.matchAll(/<([a-z][a-z0-9-]*)\b([^>]*)>/gi)];
class Element {
  constructor(tag, attrs) {
    this.tag = tag;
    this.attributes = Object.fromEntries([...attrs.matchAll(/([\w-]+)="([^"]*)"/g)].map(m => [m[1], m[2]]));
    this.dataset = Object.fromEntries(Object.entries(this.attributes).filter(([k])=>k.startsWith('data-')).map(([k,v])=>[k.slice(5).replace(/-([a-z])/g, (_, x)=>x.toUpperCase()),v]));
    this.hidden = /\shidden\b/.test(attrs);
    this.style = {};
    this.value = this.attributes.value || '';
    this.textContent = '';
    this.children = [];
    const classes = new Set((this.attributes.class || '').split(' ').filter(Boolean));
    this.classList = {
      contains: c => classes.has(c),
      add: c => classes.add(c),
      remove: c => classes.delete(c),
      toggle(c, force) { const on = force === undefined ? !classes.has(c) : !!force; if (on) classes.add(c); else classes.delete(c); return on; },
    };
  }
  setAttribute(k,v) { this.attributes[k] = v; }
  getAttribute(k) { return this.attributes[k]; }
  replaceChildren() { this.children=[]; this.value=''; }
  append(el) { this.children.push(el); if(this.children.length===1)this.value=el.value; }
  addEventListener() {}
}
const elements = tags.map(m => new Element(m[1], m[2]));
const ids = new Map();
for (const e of elements) {
  if (!e.attributes.id) continue;
  assert(!ids.has(e.attributes.id), '重复ID：'+e.attributes.id);
  ids.set(e.attributes.id,e);
}
const document = {
  documentElement: { dataset: {} },
  getElementById: id => { assert(ids.has(id),'缺失ID：'+id); return ids.get(id); },
  createElement: tag => new Element(tag,''),
  querySelectorAll(selector) {
    const selectors = selector.split(',').map(s=>s.trim());
    return elements.filter(e=>selectors.some(s=>s.startsWith('.') ? e.classList.contains(s.slice(1)) : s.startsWith('[') ? Object.hasOwn(e.attributes,s.slice(1,-1)) : e.tag===s));
  },
};
// 填入浏览器从HTML解析得到的名称，不假定空textContent是页面真实状态。
ids.get('primary-voice-name').textContent = '语音输入 1';
const ctx = vm.createContext({ document, matchMedia: () => ({matches:false}), setTimeout:()=>1,clearTimeout:()=>{},window:{},navigator:{} });
vm.runInContext(script,ctx);
const evaluate = s => vm.runInContext(s,ctx);
const home = html.match(/<section class="page" data-page="general">([\s\S]*?)<\/section>/)[1];
assert.equal(evaluate('activePage'),'general');
assert(home.indexOf('id="primary-voice-name"') < home.indexOf('id="optimization-switch"'));
assert(home.indexOf('id="secondary-voice-row"') < home.indexOf('id="optimization-switch"'));
assert(home.includes('文本强化'));
assert(home.includes('（语音识别文本＋强化规则＝最终文本）'));
assert(!home.includes('已开启') && !home.includes('已关闭'));
assert(html.includes('id="strengthening-navigation"'));
assert.equal((html.match(/id="optimization-switch"/g)||[]).length,1);
assert(!html.includes('optimization-state'));
assert(!html.includes('edit-prompts'));
assert(home.includes('id="secondary-voice-row" data-opt-gated=""'));
assert(!html.includes('已开启') && !html.includes('已关闭'));
assert(html.includes('class="content-title optimization-heading">文本强化规则'));
assert(html.includes('#strengthening-navigation { border-left: 2px solid var(--accent)'));
const transcriptionPage = html.match(/<section class="page" data-page="voice-prompts"[^>]*>([\s\S]*?)<\/section>/)[1];
const strengtheningPage = html.match(/<section class="page" data-page="strengthening-prompts"[^>]*>([\s\S]*?)<\/section>/)[1];
assert(transcriptionPage.includes('语音识别规则'));
assert(transcriptionPage.includes('角色定义') && transcriptionPage.includes('转录规则') && transcriptionPage.includes('专有词汇'));
assert(!transcriptionPage.includes('voice-template-editor'));
assert(strengtheningPage.includes('文本强化规则') && strengtheningPage.includes('voice-template-editor'));
assert(!strengtheningPage.includes('prompt-agent') && !strengtheningPage.includes('prompt-rules'));
assert.equal(ids.get('secondary-voice-row').hidden,false);
const styles = ['shortcut-template','shortcut2-template'].map(id=>ids.get(id).value);
const primaryName = ids.get('primary-voice-name');
primaryName.textContent='工作语音';
evaluate('setOptimization(false)');
assert.equal(ids.get('optimization-switch').getAttribute('aria-checked'),'false');
assert.equal(primaryName.textContent,'语音输入');
assert.equal(ids.get('secondary-voice-row').hidden,true);
for (const e of document.querySelectorAll('[data-opt-gated]')) assert.equal(e.hidden,true);
assert.equal(ids.get('optimization-switch').hidden,false);
assert.equal(primaryName.hidden,false);
evaluate("go('voice-prompts')");
assert.equal(evaluate('activePage'),'general');
evaluate("go('strengthening-prompts')");
assert.equal(evaluate('activePage'),'general');
for(const id of ['models','extract','extract-prompts','voice-learning','history','usage','backup','about']) {
  evaluate(`go('${id}')`); assert.equal(evaluate('activePage'),id);
}
evaluate('setOptimization(true)');
assert.equal(primaryName.textContent,'工作语音');
assert.equal(ids.get('secondary-voice-row').hidden,false);
for (const e of document.querySelectorAll('[data-opt-gated]')) assert.equal(e.hidden,false);
assert.deepEqual(['shortcut-template','shortcut2-template'].map(id=>ids.get(id).value),styles);
evaluate("go('voice-prompts')");
assert.equal(evaluate('activePage'),'voice-prompts');
evaluate("go('strengthening-prompts')");
assert.equal(evaluate('activePage'),'strengthening-prompts');
evaluate('setOptimization(false)');
assert.equal(evaluate('activePage'),'general');
evaluate('setOptimization(true)');
assert.equal(primaryName.textContent,'工作语音');
assert(!html.includes("classList.toggle('open')"));
console.log('通过：两个语音入口均在开关上方、转录与强化提示词独立页面、强化入口配色、公式文案一致、开关仅保留按钮、关闭隐藏风格和第二语音入口、主快捷键独立显示、重新开启恢复名称和风格、两类提示词联动、入口配色标记、12个页面导航、脚本语法。');

assert(html.includes('>语音识别</div>'));
assert(html.includes('>语音识别设置</button>'));
assert(html.includes('>语音识别规则</button>'));
assert(html.includes('>文本强化规则</button>'));
console.log('通过：侧栏与页面名称统一，规则正文保持原样。');
