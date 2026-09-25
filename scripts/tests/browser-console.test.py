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
  const unity = 'Error\n at build.wasm.UnityGUID::Init()\n at build.wasm.AssetBundleLoadFromStreamAsyncOperation::TryInitializeMemoryCache()';
  const cases = [
    {name:'verified Unity startup', stack:unity, message:warning, passes:true},
    {name:'unidentified blocking', stack:'Error\n at another_operation()', message:warning},
    {name:'other GUID caller', stack:'Error\n at build.wasm.UnityGUID::Init()', message:warning},
    {name:'runtime blocking', stack:unity, message:warning, connected:true},
    {name:'ordinary error', stack:unity, message:'Storage transaction failed'},
    {name:'missing stack', stack:'', message:warning},
    {name:'ordinary loader progress', message:'dependency: loading-workers', passes:true},
  ];
  for (const scenario of cases) {
    const listeners = new Map();
    const emit = (type, text) => listeners.get('console')({type:()=>type,text:()=>text});
    const page = {
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
          realm.console.error(scenario.message);
        };
      },
    };
    if(scenario.passes) {
      const result=await check(page);
      assert.equal(result.status,'passed',scenario.name);
      assert.equal(result.startupProgress.length,1,scenario.name);
      if(scenario.stack) assert.ok(result.startupProgress[0].includes(scenario.stack));
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
