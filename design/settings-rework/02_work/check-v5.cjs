const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const path = require('node:path');
const output = path.resolve(__dirname, '../03_output/20261015-1213-设置界面v5-独立开关联动提示词.html');
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
const ctx = vm.createContext({ document, matchMedia: () => ({matches:false}), setTimeout:()=>1,clearTimeout:()=>{},window:{},navigator:{} });
vm.runInContext(script,ctx);
const evaluate = s => vm.runInContext(s,ctx);
assert.equal(evaluate('activePage'),'general');
const home = html.match(/<section class="page" data-page="general">([\s\S]*?)<\/section>/)[1];
assert(home.indexOf('id="optimization-switch"') < home.indexOf('id="shortcut-template"'), '开关必须放在快捷键之前');
assert.equal((html.match(/id="optimization-switch"/g)||[]).length,1);
const prompts = html.match(/<section class="page" data-page="voice-prompts"[^>]*>([\s\S]*?)<\/section>/)[1];
assert(prompts.includes('转写提示词') && prompts.includes('文本优化提示词'));
assert.equal(ids.get('prompt-navigation').hidden,false);
assert.equal(ids.get('voice-prompt-page').hidden,false);
evaluate('setOptimization(false)');
assert.equal(ids.get('optimization-switch').getAttribute('aria-checked'),'false');
assert.equal(ids.get('optimization-state').textContent,'已关闭');
for (const e of document.querySelectorAll('[data-opt-gated]')) assert.equal(e.hidden,true);
assert.equal(ids.get('optimization-switch').hidden,false);
evaluate("go('voice-prompts')");
assert.equal(evaluate('activePage'),'general');
for(const id of ['models','extract','extract-prompts','voice-learning','history','usage','backup','about']) {
  evaluate(`go('${id}')`); assert.equal(evaluate('activePage'),id);
}
evaluate('setOptimization(true)');
for (const e of document.querySelectorAll('[data-opt-gated]')) assert.equal(e.hidden,false);
evaluate("go('voice-prompts')");
assert.equal(evaluate('activePage'),'voice-prompts');
evaluate('setOptimization(false)');
assert.equal(evaluate('activePage'),'general');
evaluate('setOptimization(true)');
assert.equal(ids.get('optimization-switch').getAttribute('aria-checked'),'true');
assert(!html.includes("classList.toggle('open')"), '不应有手动折叠');
assert(!html.includes('<option value="">无</option>'));
assert(!html.includes('后面也不需要') && !html.includes('第 1 步') && !html.includes('第 2 步'));
console.log('通过：HTML脚本语法、ID唯一、首页开关位置、两类提示词显示/隐藏、关闭时导航返回、重新开启恢复、11个页面导航。');
