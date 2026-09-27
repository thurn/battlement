#!/usr/bin/env python3
"""Keep browser startup warning classification narrow and observable."""

import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from web_compatibility import _check_code


RUNNER = r"""
const assert = require('node:assert/strict');
const vm = require('node:vm');
let input = '';
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', async () => {
  const check = eval('(' + JSON.parse(input) + ')');
  const warning = 'Blocking on the main thread is very dangerous, see https://emscripten.org/docs/porting/pthreads.html#blocking-on-the-main-browser-thread';
  const pacing = 'Looks like you are rendering without using requestAnimationFrame for the main loop. You should use 0 for the frame rate in emscripten_set_main_loop in order to use requestAnimationFrame, as that can greatly improve your frame rates!';
  const unity = 'Error\n at build.wasm.UnityGUID::Init()\n at build.wasm.AssetBundleLoadFromStreamAsyncOperation::TryInitializeMemoryCache()';
  const cases = [
    {name:'verified Unity startup', stack:unity, message:warning, passes:true},
    {name:'unidentified blocking', stack:'Error\n at another_operation()', message:warning},
    {name:'other GUID caller', stack:'Error\n at build.wasm.UnityGUID::Init()', message:warning},
    {name:'verified Unity reload', stack:unity, message:warning, connected:true, navigation:'main', passes:true},
    {name:'subframe keeps runtime boundary', stack:unity, message:warning, connected:true, navigation:'child'},
    {name:'reconnected runtime blocking', stack:unity, message:warning, connected:true, navigation:'main', reconnected:true},
    {name:'runtime blocking', stack:unity, message:warning, connected:true},
    {name:'ordinary error', stack:unity, message:'Storage transaction failed'},
    {name:'missing stack', stack:'', message:warning},
    {name:'ordinary loader progress', message:'dependency: loading-workers', passes:true},
    {name:'capped frame loop', stack:'Error\n at Object.MainLoop_runner [as runner] (framework.js:3559)', message:pacing, connected:true, passes:true, advisory:true},
    {name:'unidentified pacing warning', stack:'Error\n at another_operation()', message:pacing},
    {name:'modified pacing warning', stack:'Error\n at MainLoop_runner (framework.js:3559)', message:pacing+' unexpected failure'},
  ];
  for (const scenario of cases) {
    const listeners = new Map();
    const emit = (type, text) => listeners.get('console')({type:()=>type,text:()=>text});
    const mainFrame = {};
    const page = {
      mainFrame() { return mainFrame; },
      on(name, callback) { listeners.set(name, callback); },
      off(name) { listeners.delete(name); },
      setDefaultTimeout() {},
      async screenshot() {},
      async addInitScript(callback, argument) {
        const realm = {
          console:{error:(...args)=>emit('error',args.join(' '))},
          Error: class { constructor() { this.stack=scenario.stack; } },
        };
        vm.runInNewContext('('+callback+')('+JSON.stringify(argument)+')',realm);
        this.runMessages = () => {
          if(scenario.connected) emit('log','battlement.host.connected');
          if(scenario.navigation) listeners.get('framenavigated')(scenario.navigation === 'main' ? mainFrame : {});
          if(scenario.reconnected) emit('log','battlement.host.connected');
          realm.console.error(scenario.message);
        };
      },
    };
    if(scenario.passes) {
      const result=await check(page);
      assert.equal(result.status,'passed',scenario.name);
      const evidence = scenario.advisory ? result.runtimeAdvisories : result.startupProgress;
      assert.equal(evidence.length,1,scenario.name);
      if(scenario.stack) assert.ok(evidence[0].includes(scenario.stack));
    } else {
      await assert.rejects(check(page),undefined,scenario.name);
    }
    assert.equal(listeners.size,0,scenario.name+' cleans up observers');
  }
  console.log(cases.length+' browser console classification cases passed');
}).on('error',error=>{throw error;});
"""


if __name__ == "__main__":
    contract = """async page => {
      page.runMessages();
      return {interaction:'Startup diagnostic boundary',assertions:1};
    }"""
    source = _check_code(contract, "http://fixture/", "/tmp/fixture", "null")
    subprocess.run(["node", "-e", RUNNER], input=json.dumps(source), text=True, check=True)
