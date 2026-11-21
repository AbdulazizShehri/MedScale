import test from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import {fileURLToPath} from "node:url";
import {dirname, resolve} from "node:path";
import {runInNewContext} from "node:vm";
import {evaluateCareLoop} from "../src/care-loop.mjs";
import {makeSyntheticBundle} from "../fixtures/synthetic.mjs";

const dir=dirname(fileURLToPath(import.meta.url));
const scenarios=[
 "complete","missing-result","overdue","cancelled","wrong-patient","missing-provenance"
];
class FakeElement {
 constructor(tag="div",scenario=null) {
  this.tag=tag;this.dataset=scenario?{scenario}:{};this.textContent="";
  this.children=[];this.attributes=new Map();this.handlers=new Map();
 }
 replaceChildren(...nodes){this.children=[...nodes]}
 append(...nodes){this.children.push(...nodes)}
 setAttribute(name,val){this.attributes.set(name,String(val))}
 addEventListener(name,cb){this.handlers.set(name,cb)}
 click(){const listener=this.handlers.get("click");assert.ok(listener);listener()}
}
async function loadUI(){
 const html=await readFile(resolve(dir,"../index.html"),"utf8");
 const code=html.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];
 assert.ok(code,"HTML must contain application module");
 const script=code.replace(/^import \{evaluateCareLoop\} from "\.\/src\/care-loop\.mjs";\s*/m,"")
   .replace(/^import \{makeSyntheticBundle\} from "\.\/fixtures\/synthetic\.mjs";\s*/m,"");
 assert.ok(!script.includes('import {'),"unexpected additional imports");
 const elements=new Map(["status","closure","stages","evidence","warning"].map(id=>[id,new FakeElement()]));
 const buttons=scenarios.map(name=>new FakeElement("button",name));
 const document={
  getElementById(id){assert.ok(elements.has(id),"Unexpected DOM node "+id);return elements.get(id)},
  createElement(tag){return new FakeElement(tag)},
  querySelectorAll(selector){assert.equal(selector,"[data-scenario]");return buttons}
 };
 runInNewContext(script,{document,evaluateCareLoop,makeSyntheticBundle},{timeout:3000});
 return {elements,buttons};
}
const status=(dom)=>dom.elements.get("status").textContent;
const warning=(dom)=>dom.elements.get("warning").textContent;

test("actual HTML UI module renders complete synthetic case",async()=>{
 const dom=await loadUI();
 assert.equal(status(dom),"CLOSED");
 assert.match(dom.elements.get("closure").textContent,/all seven human-evidenced stages/);
 assert.equal(dom.elements.get("stages").children.length,7);
 assert.equal(dom.elements.get("evidence").children.length,7);
 assert.equal(dom.buttons.filter(b=>b.attributes.get("aria-pressed")==="true").length,1);
 assert.equal(dom.buttons[0].attributes.get("aria-pressed"),"true");
});
test("six UI controls each render expected fail-closed status and a warning",async()=>{
 const dom=await loadUI();
 const expected={
  "complete":"CLOSED",
  "missing-result":"ATTENDED",
  "overdue":"OVERDUE",
  "cancelled":"BLOCKED",
  "wrong-patient":"BLOCKED",
  "missing-provenance":"BLOCKED"
 };
 for(const button of dom.buttons){
  button.click();
  assert.equal(status(dom),expected[button.dataset.scenario],button.dataset.scenario);
  assert.equal(dom.buttons.filter(b=>b.attributes.get("aria-pressed")==="true").length,1);
  assert.equal(button.attributes.get("aria-pressed"),"true");
  if(button.dataset.scenario!=="complete"){
   assert.match(dom.elements.get("closure").textContent,/NOT CLOSED/);
   assert.ok(warning(dom).length>10,button.dataset.scenario+" must explain why closure failed");
  }
 }
});
test("untrusted values are written as text, not HTML",async()=>{
 const dom=await loadUI();
 for(const item of dom.elements.get("evidence").children){
  assert.equal(item.children.length,1);
  assert.equal(item.children[0].tag,"code");
  assert.match(item.children[0].textContent,/^[A-Za-z]+\//);
 }
});
