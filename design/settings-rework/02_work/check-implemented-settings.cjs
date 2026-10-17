const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const esbuild = require('esbuild');
const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');
const source = process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(__dirname, '../../..');
global.navigator = { platform: 'MacIntel' };
const fixture = {
  general: { shortcut: 'F4', shortcut2: 'F5', extractShortcut: 'F6', extractShortcut2: 'F7', shortcutTemplate: 'voice-optimize', shortcut2Template: 'voice-translate', extractShortcutTemplate: 'image-extract', extractShortcut2Template: 'image-translate', textStrengtheningEnabled: true, shortcutLabel: '旧自定义语音', shortcut2Label: '旧自定义语音2', extractShortcutLabel: '旧截图名', extractShortcut2Label: '旧截图名2', theme: 'system', microphone: 'system-default', maxRecordingSeconds: 180, launchAtLogin: false, pttMode: false, overwriteClipboard: true },
  localApi: { enabled: false, port: 8765 },
  models: { builtinApiKeys: { gemini:'', deepseek:'', dashscope:'', openrouter:'', mimo:'' }, custom:[] },
  transcribe: { modelId:'builtin-gemini-3.8-flash', thinking:{enabled:false,level:'LOW'}, prompts:{agent:'',rules:'',vocabulary:''},injectTextRules:false },
  voiceTemplates: { modelId:'builtin-deepseek-flash', thinking:{enabled:false,level:'LOW'}, templates:[{id:'voice-optimize',name:'自动换行',prompt:''},{id:'voice-translate',name:'翻译',prompt:''},{id:'voice-custom',name:'自定义',prompt:''}],reuseTranscribeReferences:false },
  voiceLearning:{modelId:'builtin-deepseek-flash',thinking:{enabled:false,level:'LOW'}},
  extract: {templates:[{id:'image-extract',name:'文字识别',prompt:''},{id:'image-translate',name:'翻译',prompt:''}]},
  advanced:{proxyEnabled:false,proxyUrl:'',transcribeTimeout:10,optimizeTimeout:10,maxRetries:3,maxParallel:3},
  backup:{s3:{endpoint:'',region:'',bucket:'',accessKey:'',secretKey:'',prefix:''}},
};
async function loadComponent(name,folder='tabs', hooks = null) {
  const result=await esbuild.build({entryPoints:[path.join(source,'src/views/settings',folder,name+'.tsx')],bundle:true,write:false,format:'cjs',platform:'node',jsx:'automatic',external:['react','react/*'],plugins:[{name:'mock-native-apis',setup(build){
    build.onResolve({filter:/tauri-api$/},()=>({path:'api',namespace:'mock'}));
    build.onLoad({filter:/.*/,namespace:'mock'},()=>({contents:'export const getLaunchAtLogin=async()=>false; export const getLocalApiStatus=async()=>({running:false,port:null,error:null}); export const onEvent=async()=>()=>{}; export const setLaunchAtLogin=async()=>{}; export const listInputDevices=async()=>[]; export const createUserPromptFile=async()=>"mock.md";',loader:'js'}));
    build.onResolve({filter:/PromptEditor$/},()=>({path:'editor',namespace:'editor-mock'}));
    build.onLoad({filter:/.*/,namespace:'editor-mock'},()=>({contents:'import React from "react"; export function PromptEditor({promptFiles}) {return React.createElement("div",{"data-prompt-files":promptFiles.map(p=>p.key).join(",")},promptFiles.map(p=>p.label).join(","));}',loader:'js'}));
  }}]});
  const module={exports:{}};
  const moduleRequire = id => id === 'react' && hooks ? {...React,...hooks} : require(id);
  vm.runInNewContext(result.outputFiles[0].text,{module,exports:module.exports,require:moduleRequire,navigator:global.navigator,console,setTimeout,clearTimeout});return module.exports[name];
}
(async()=>{
  const General=await loadComponent('GeneralTab');
  const on=renderToStaticMarkup(React.createElement(General,{config:fixture,onSave:()=>{}}));
  assert(on.indexOf('文本强化')<on.indexOf('语音输入 1'));
  assert(on.indexOf('语音输入 1')<on.indexOf('语音输入 2'));
  assert(on.includes('strengthened-voice-inputs'));
  assert(on.includes('图片识别1')&&on.includes('图片识别2'));
  assert(!on.includes('旧自定义')&&!on.includes('旧截图名')&&!on.includes('contenteditable'));
  assert(!on.includes('<option value="">无'));
  const offConfig=structuredClone(fixture);offConfig.general.textStrengtheningEnabled=false;
  const off=renderToStaticMarkup(React.createElement(General,{config:offConfig,onSave:()=>{}}));
  assert(off.indexOf('语音输入</div>')<off.indexOf('文本强化'));
  assert(!off.includes('语音输入 1')&&!off.includes('语音输入 2'));
  assert.equal((on.match(/输出风格/g)||[]).length,4);
  assert.equal((off.match(/输出风格/g)||[]).length,2);
  const Transcribe=await loadComponent('TranscribeTab');
  const transOn=renderToStaticMarkup(React.createElement(Transcribe,{config:fixture,onSave:()=>{}}));
  const transOff=renderToStaticMarkup(React.createElement(Transcribe,{config:offConfig,onSave:()=>{}}));
  assert(transOn.includes('将文本规则注入语音规则')&&transOn.includes('将语音规则注入文本规则'));
  assert(transOff.includes('将文本规则注入语音规则'));
  assert(!transOff.includes('文本强化模型')&&!transOff.includes('将语音规则注入文本规则'));
  // 文本强化开启时注入开关置灰并给出原因，关闭时可用
  assert(transOn.includes('需先关闭文本强化才能注入'));
  assert(transOn.includes('toggle disabled'));
  assert(transOff.includes('一轮完成'));
  assert(!transOff.includes('toggle disabled'));
  const Voice=await loadComponent('VoicePromptsTab');
  const voice=renderToStaticMarkup(React.createElement(Voice,{config:fixture,onSave:()=>{}}));
  assert(voice.includes('语音识别规则')&&voice.includes('agent,rules,vocabulary'));
  assert(!voice.includes('自动换行'));
  const Strengthening=await loadComponent('StrengtheningPromptsTab');
  const strengthening=renderToStaticMarkup(React.createElement(Strengthening,{config:fixture,onSave:()=>{}}));
  assert(strengthening.includes('文本强化规则')&&strengthening.includes('例如分段、翻译。'));
  assert(strengthening.includes('voice-optimize')&&!strengthening.includes('agent,rules,vocabulary'));
  const Panel = await loadComponent('TemplatePromptsPanel','components');
  const empty = structuredClone(fixture); empty.voiceTemplates.templates = [];
  const emptyPanel = renderToStaticMarkup(React.createElement(Panel,{kind:'voice',config:empty,onSave:()=>{}}));
  assert(emptyPanel.includes('添加模板'), '空列表仍可新增模板');
  const filledPanel = renderToStaticMarkup(React.createElement(Panel,{kind:'voice',config:fixture,onSave:()=>{}}));
  // 模板名即入口，内置模板不显示删除；添加模板在下拉里
  assert(filledPanel.includes('template-name-btn'), '模板名可点开下拉');
  assert(filledPanel.includes('重命名')&&!filledPanel.includes('>重命名<'), '重命名只作图标提示文本');
  assert(!filledPanel.includes('>删除<'), '内置模板不显示删除');
  assert(!filledPanel.includes('添加模板'), '有模板时添加模板收进下拉');
  const hooks={useEffect:()=>{},useRef:()=>({current:null}),useState:initial=>[typeof initial==='function'?initial():initial,()=>{}]};
  const HookGeneral=await loadComponent('GeneralTab','tabs',hooks);
  function allElements(value) {
    if(Array.isArray(value))return value.flatMap(allElements);
    if(!React.isValidElement(value))return [];
    return [value,...allElements(value.props.children)];
  }
  let savedConfig=null;
  const offTree=allElements(HookGeneral({config:offConfig,onSave:c=>{savedConfig=c;}}));
  const switchEl=offTree.find(e=>e.type?.name==='Toggle'&&e.props.onChange.name==='setStrengthening');
  assert(switchEl, '找到真实强化开关');
  switchEl.props.onChange(true);
  assert.equal(savedConfig.general.textStrengtheningEnabled,true);
  assert.equal(savedConfig.general.shortcut,'F4');
  assert.equal(savedConfig.general.shortcut2,'F5');
  assert.equal(savedConfig.general.shortcutTemplate,'voice-optimize');
  assert.equal(savedConfig.general.shortcut2Template,'voice-translate');
  const HookPanel=await loadComponent('TemplatePromptsPanel','components',hooks);
  const userConfig=structuredClone(fixture);
  userConfig.voiceTemplates.templates=[{id:'voice-user-1',name:'用户风格',prompt:'mock.md'},...fixture.voiceTemplates.templates];
  userConfig.general.shortcutTemplate='voice-user-1';userConfig.general.shortcut2Template='voice-user-1';
  const panelTree=allElements(HookPanel({kind:'voice',config:userConfig,onSave:c=>{savedConfig=c;}}));
  const deleteButton=panelTree.find(e=>e.type==='button'&&e.props['aria-label']==='删除模板');
  assert(deleteButton,'自定义模板显示删除图标');deleteButton.props.onClick();
  assert(!savedConfig.voiceTemplates.templates.some(t=>t.id==='voice-user-1'));
  assert.equal(savedConfig.general.shortcutTemplate,'voice-optimize');
  assert.equal(savedConfig.general.shortcut2Template,'voice-optimize');
  const app=fs.readFileSync(path.join(source,'src/views/settings/App.tsx'),'utf8');
  assert(app.includes("id: 'voice-prompts', label: '语音识别规则'"));
  assert(app.includes("id: 'strengthening-prompts', label: '文本强化规则', strengtheningOnly: true"));
  console.log('通过：真实React组件在强化开/关状态下的语义输出、固定名称、入口顺序、双向规则增强、规则页拆分、侧栏门禁。未启动浏览器或修改本机设置。');
})().catch(e=>{console.error(e);process.exitCode=1;});
