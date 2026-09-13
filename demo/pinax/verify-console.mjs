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
  const results={};
  for(const action of ['ontology','navigator','qglake','semantic','discover','plan','execute','deny-column','deny-purpose']) {
    await page.locator(`button[data-action="${action}"]`).click();
    await page.waitForFunction(()=>!document.querySelector('button').disabled,{},{timeout:130000});
    const value=JSON.parse(await page.locator('#output').innerText());
    assert.equal(value.status,action.startsWith('deny-')?'denied':'passed',JSON.stringify(value));
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
  await page.setViewportSize({width:390,height:844});
  await page.screenshot({path:join(output,'console-mobile.png'),fullPage:true});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  assert.deepEqual(errors,[]);
  await writeFile(join(output,'results.json'),JSON.stringify({browser_errors:errors,actions:results},null,2)+'\n');
  console.log('All nine live browser actions passed; desktop/mobile screenshots saved.');
} finally {await browser.close();}
