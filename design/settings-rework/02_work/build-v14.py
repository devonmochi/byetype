from pathlib import Path

root = Path(__file__).resolve().parent.parent
source = root / '03_output/20261015-1401-设置界面v13-固定语音入口名称.html'
output = root / '03_output/20261015-1411-设置界面v14-固定图片识别名称.html'
html = source.read_text()

def replace(old, new):
    global html
    assert html.count(old) == 1, f'预期唯一匹配：{old[:100]}'
    html = html.replace(old, new, 1)

replace('<title>ByeType设置界面对齐稿v13</title>', '<title>ByeType设置界面对齐稿v14</title>')
replace('<span contenteditable="true" aria-label="截图取词名称">截图取词</span>', '<span id="image-input-name-1">图片识别1</span>')
replace('<span contenteditable="true" aria-label="截图翻译名称">截图翻译</span>', '<span id="image-input-name-2">图片识别2</span>')
replace('aria-label="修改截图取词快捷键"', 'aria-label="修改图片识别1快捷键"')
replace('aria-label="修改截图翻译快捷键"', 'aria-label="修改图片识别2快捷键"')
assert not output.exists(), '不覆盖已交付稿'
output.write_text(html)

check = (root / '02_work/check-v13.cjs').read_text().replace(source.name, output.name)
check += '''
assert.equal(get('image-input-name-1').getAttribute('contenteditable'),undefined);
assert.equal(get('image-input-name-2').getAttribute('contenteditable'),undefined);
assert(html.includes('<span id="image-input-name-1">图片识别1</span>'));
assert(html.includes('<span id="image-input-name-2">图片识别2</span>'));
assert(!html.includes('截图取词名称')&&!html.includes('截图翻译名称'));
assert(get('extract-template')&&get('extract2-template'));
console.log('通过：图片识别1、图片识别2名称固定不可编辑，快捷键和输出风格保留。');
'''
check_path = root / '02_work/check-v14.cjs'
assert not check_path.exists()
check_path.write_text(check)
print(output)
