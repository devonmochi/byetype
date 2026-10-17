const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const path = require('node:path');
const html = fs.readFileSync(path.resolve(__dirname,'../03_output/20261015-1307-设置界面v10-强化输入缩进.html'),'utf8');
const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
new vm.Script(script);
class Element {
  constructor(tag, attrs = '') {
    this.tag = tag;
    this.attributes = Object.fromEntries([...attrs.matchAll(/([\w-]+)="([^"]*)"/g)].map(m=>[m[1],m[2]]));
    this.dataset = Object.fromEntries(Object.entries(this.attributes).filter(([k])=>k.startsWith('data-')).map(([k,v])=>[k.slice(5).replace(/-([a-z])/g,(_,x)=>x.toUpperCase()),v]));
    this.hidden = /\shidden\b/.test(attrs);
    this.value = this.attributes.value || '';
    this.textContent = '';
    this.children = [];
    this.parentElement = null;
    this.style = {};
    const classes = new Set((this.attributes.class || '').split(' ').filter(Boolean));
    this.classList = {
      contains: c=>classes.has(c), add: c=>classes.add(c), remove: c=>classes.delete(c),
      toggle(c,force) {const on=force===undefined?!classes.has(c):!!force;if(on)classes.add(c);else classes.delete(c);return on;},
    };
  }
  setAttribute(k,v){this.attributes[k]=v;}
  getAttribute(k){return this.attributes[k];}
  addEventListener(){}
  appendChild(child){return this.insertBefore(child,null);}
  append(child){this.appendChild(child);if(this.tag==='select' && this.children.length===1)this.value=child.value;}
  insertBefore(child,ref){
    if(ref===child)return child;
    if(child.parentElement){const old=child.parentElement;old.children.splice(old.children.indexOf(child),1);}
    const index=ref===null?this.children.length:this.children.indexOf(ref);
    assert(index>=0,'参照节点必须属于目标父节点');
    this.children.splice(index,0,child);child.parentElement=this;return child;
  }
  replaceChildren(){this.children.forEach(c=>{c.parentElement=null;});this.children=[];this.value='';}
}
const elements=[];
const root=new Element('root');
const stack=[root];
const voids=new Set(['area','base','br','col','embed','hr','img','input','link','meta','param','source','track','wbr']);
// 不将提示词、脚本中的文字当成DOM标签。
const markup=html.replace(/<script>[\s\S]*?<\/script>/,'').replace(/<textarea([^>]*)>[\s\S]*?<\/textarea>/g,'<textarea$1></textarea>');
for(const match of markup.matchAll(/<(\/?)([a-z][a-z0-9-]*)\b([^>]*)>/gi)){
  const [,closing,tag,attrs]=match;
  if(closing){assert.equal(stack.at(-1).tag,tag,'标签闭合顺序');stack.pop();continue;}
  const e=new Element(tag,attrs);elements.push(e);stack.at(-1).appendChild(e);
  if(!voids.has(tag) && !/\/\s*$/.test(attrs))stack.push(e);
}
assert.equal(stack.length,1,'标签完整闭合');
const ids=new Map();
for(const e of elements){const id=e.attributes.id;if(!id)continue;assert(!ids.has(id),'重复ID：'+id);ids.set(id,e);}
const document={
  documentElement:{dataset:{}},
  getElementById(id){assert(ids.has(id),'缺失ID：'+id);return ids.get(id);},
  createElement(tag){return new Element(tag);},
  querySelectorAll(selector){const ss=selector.split(',').map(s=>s.trim());return elements.filter(e=>ss.some(s=>s.startsWith('.')?e.classList.contains(s.slice(1)):s.startsWith('[')?Object.hasOwn(e.attributes,s.slice(1,-1)):e.tag===s));},
};
ids.get('primary-voice-name').textContent='语音输入 1';
const ctx=vm.createContext({document,matchMedia:()=>({matches:false}),setTimeout:()=>1,clearTimeout:()=>{},window:{},navigator:{}});
vm.runInContext(script,ctx);
const evaluate=s=>vm.runInContext(s,ctx);
const get=id=>ids.get(id);
const primary=get('primary-voice-row');
const secondary=get('secondary-voice-row');
const nested=get('strengthened-voice-inputs');
const plain=get('plain-voice-slot');
const mode=get('optimization-row');
const styles=['shortcut-template','shortcut2-template'].map(id=>get(id).value);
assert.equal(evaluate('activePage'),'general');
assert.equal(primary.parentElement,nested);
assert.equal(secondary.parentElement,nested);
assert.deepEqual(nested.children,[primary,secondary]);
assert(mode.parentElement.children.indexOf(mode)<mode.parentElement.children.indexOf(nested),'开启时两个输入在强化下方');
assert.equal(plain.hidden,true);
assert(html.includes('.strengthened-voice-inputs { margin-left: 32px;'));
get('primary-voice-name').textContent='工作语音';
evaluate('setOptimization(false)');
assert.equal(primary.parentElement,plain);
assert.equal(plain.hidden,false);
assert(plain.parentElement.children.indexOf(plain)<plain.parentElement.children.indexOf(mode),'关闭时独立入口在开关上方');
assert.equal(get('primary-voice-name').textContent,'语音输入');
assert.equal(secondary.hidden,true);
assert.equal(nested.hidden,true);
for(const e of document.querySelectorAll('[data-opt-gated]'))assert.equal(e.hidden,true);
evaluate("go('strengthening-prompts')");assert.equal(evaluate('activePage'),'general');
for(let i=0;i<3;i++){
  evaluate('setOptimization(true)');
  assert.equal(primary.parentElement,nested);
  assert.deepEqual(nested.children,[primary,secondary]);
  assert.equal(plain.hidden,true);
  assert.equal(get('primary-voice-name').textContent,'工作语音');
  for(const e of document.querySelectorAll('[data-opt-gated]'))assert.equal(e.hidden,false);
  assert.deepEqual(['shortcut-template','shortcut2-template'].map(id=>get(id).value),styles);
  evaluate("go('strengthening-prompts')");assert.equal(evaluate('activePage'),'strengthening-prompts');
  evaluate('setOptimization(false)');assert.equal(evaluate('activePage'),'general');
  assert.equal(primary.parentElement,plain);
}
evaluate('setOptimization(true)');
const recognition=html.match(/<section class="page" data-page="voice-prompts"[^>]*>([\s\S]*?)<\/section>/)[1];
const strengthening=html.match(/<section class="page" data-page="strengthening-prompts"[^>]*>([\s\S]*?)<\/section>/)[1];
assert(recognition.includes('语音识别规则')&&!recognition.includes('voice-template-editor'));
assert(strengthening.includes('文本强化规则')&&strengthening.includes('voice-template-editor'));
assert(!html.includes('optimization-state')&&!html.includes('已开启')&&!html.includes('已关闭'));
for(const id of ['transcribe','voice-prompts','strengthening-prompts','models','extract','extract-prompts','voice-learning','history','usage','backup','about']){evaluate(`go('${id}')`);assert.equal(evaluate('activePage'),id);}
console.log('通过：关闭时单独语音输入在前；开启时输入1、输入2缩进到强化下方；多次切换保留快捷键、名称与风格；规则页面联动和导航；脚本语法与DOM结构。');
