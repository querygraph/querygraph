import {createRequire} from 'node:module';
import {mkdir, writeFile} from 'node:fs/promises';
import {homedir} from 'node:os';
import {resolve, join} from 'node:path';
import {pathToFileURL} from 'node:url';

const source = process.argv[2] ?? 'http://localhost:18081/slides?format=iceberg&part=all';
const output = resolve(process.argv[3] ?? 'demo/pinax/dist');
const press = process.env.FIRSTPAIR_ROOT ?? join(homedir(), 'src/firstpair');
const require = createRequire(join(press, 'package.json'));
const {chromium} = require('playwright');
await mkdir(output, {recursive:true});
const browser = await chromium.launch({headless:true});
try {
  const page = await browser.newPage({viewport:{width:1280,height:720}});
  await page.goto(/^https?:/.test(source) ? source : pathToFileURL(resolve(source)).href);
  await page.evaluate(()=>document.fonts.ready);
  const count = await page.locator('.slide').count();
  const issues = [];
  for (let index=0;index<count;index++) {
    await page.evaluate(index=>show(index),index);
    const overflow = await page.locator('.slide.active').evaluate(slide=>{
      const frame = slide.getBoundingClientRect();
      return [...slide.querySelectorAll('h1,h2,p,pre,li,table,img')].filter(element=>{
        const box=element.getBoundingClientRect();
        return box.left<frame.left-1 || box.right>frame.right+1 || box.top<frame.top-1 || box.bottom>frame.bottom+1;
      }).map(element=>element.textContent.slice(0,100));
    });
    if(overflow.length)issues.push({slide:index+1,overflow});
    await page.screenshot({path:join(output,`slide-${String(index+1).padStart(2,'0')}.png`)});
  }
  if(issues.length)throw new Error(JSON.stringify(issues));
  await page.locator('a').evaluateAll(links=>links.forEach(link=>{
    const url=new URL(link.href);
    if(['localhost','127.0.0.1'].includes(url.hostname)) {
      url.protocol='https:';url.host='demo.rust.ai';url.port='';link.href=url.href;
    }
  }));
  await page.pdf({path:join(output,'querygraph-pinax-slides.pdf'),preferCSSPageSize:true,printBackground:true});
  await writeFile(join(output,'validation.json'),JSON.stringify({slides:count,overflow:issues},null,2)+'\n');
  console.log(`Rendered ${count} slides to ${output}`);
} finally {
  await browser.close();
}
