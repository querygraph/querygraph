// Verify the actual EC2 console through an established SSH tunnel.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdir, writeFile} from 'node:fs/promises';
import {homedir} from 'node:os';
import {join, resolve} from 'node:path';
const require = createRequire(join(process.env.FIRSTPAIR_ROOT ?? join(homedir(),'src/firstpair'),'package.json'));
const {chromium}=require('playwright');
const output=resolve(process.argv[2] ?? '/tmp/querygraph-pinax-console-check');
await mkdir(output,{recursive:true});
const browser=await chromium.launch({headless:true});
try {
  const page=await browser.newPage({viewport:{width:1440,height:1050}});
  const errors=[];
  page.on('pageerror',error=>errors.push(String(error)));
  await page.goto('http://localhost:18081');
  await page.screenshot({path:join(output,'console-desktop.png'),fullPage:true});
  // Regression: component cartouches must explain themselves and select a step.
  for (const name of ['QueryGraph','Sail','LakeCat','Pinax','TypeSec','Grust','Marciana','MCP']) {
    await page.locator('[data-component="'+name+'"]').click();
    assert.ok((await page.locator('#result-title').innerText()).startsWith(name+' ·'));
    assert.equal(await page.locator('article.selected').count(),1);
    assert.ok(await page.locator('#summary a').getAttribute('href'));
  }
  const results={};
  for(const action of ['lakehouse','standards','discover','ontology','mcp','plan','execute','semantic','navigator','qglake','deny-column','deny-purpose']) {
    if(action.startsWith('deny-')) await page.locator('.controls > details').evaluate(e=>e.open=true);
    await page.locator('button[data-action="'+action+'"]').click();
    // Regression: current result and selected step remain aligned and visible.
    const layout=await page.evaluate(()=>{
      const r=document.querySelector('.result').getBoundingClientRect();
      const c=document.querySelector('.controls').getBoundingClientRect();
      const a=document.querySelector('article.selected').getBoundingClientRect();
      return {left:r.right<=c.left,visible:r.top>=0&&r.bottom<=innerHeight,aligned:Math.abs(a.top-c.top)<3,outer:window.scrollY};
    });
    assert.deepEqual(layout,{left:true,visible:true,aligned:true,outer:0});
    await page.waitForFunction(()=>!document.querySelector('button').disabled,{},{timeout:130000});
    const text=await page.locator('#output').textContent();
    assert.ok(text,action+': '+await page.locator('#status').innerText()+' '+await page.locator('#summary').innerText());
    assert.ok(!(await page.locator('#status').innerText()).includes('failed'),await page.locator('#summary').innerText());
    const value=JSON.parse(text);
    console.log('Verified action:',action);
    assert.equal(value.status,action.startsWith('deny-')?'denied':'passed',JSON.stringify(value));
    if(action==='lakehouse') {assert.equal(value.result.catalog,'LakeCat');assert.equal(value.result.catalog_plan.table,'rows');assert.equal(value.result.schema.fields.length,3);}
    if(action==='standards') assert.deepEqual(value.result.tables.map(t=>t.name).sort(),['customers','employees','transactions']);
    if(action==='mcp') assert.deepEqual(value.result.execution.result.structuredContent.rows,[{id:2}]);
    if(action==='ontology') { assert.ok(value.result.search.matches.length > 0); assert.ok(value.result.search.ontology_digest.startsWith('sha256:')); }
    if(['navigator','qglake','semantic'].includes(action)) assert.ok(value.ontology_consultation.search.ontology_digest.startsWith('sha256:'));
    if(action==='execute')assert.deepEqual(value.result.rows,[{id:2}]);
    if(action.startsWith('deny-'))assert.equal(value.rows_released,false);
    if(action==='semantic'){
      assert.equal(value.result.sail.graph.loaded_nodes,34);
      assert.equal(value.result.sail.graph.loaded_edges,33);
      assert.equal(value.result.sail.graph.verified_node_label,'DataverseDataset');
    }
    results[action]=value;
  }
  await page.screenshot({path:join(output,'console-desktop-result.png')});
  await page.setViewportSize({width:390,height:844});
  await page.screenshot({path:join(output,'console-mobile.png'),fullPage:true});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.locator('[data-component="Grust"]').click();
  assert.ok((await page.locator('#result-title').innerText()).startsWith('Grust'));
  assert.equal(await page.evaluate(()=>document.querySelector('.controls').scrollHeight>document.querySelector('.controls').clientHeight),true);
  assert.deepEqual(errors,[]);
  await writeFile(join(output,'results.json'),JSON.stringify({browser_errors:errors,actions:results},null,2)+'\n');
  console.log('All twelve live browser actions and component/layout regressions passed; desktop/mobile screenshots saved.');
} finally {await browser.close();}
