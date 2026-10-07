import { existsSync } from "node:fs";
import { resolve } from "node:path";
import assert from "node:assert/strict";
// External tauri-driver only. No automation endpoint or test plugin is built into the app.
const binary = resolve(
  process.env.CAD_TEST_BINARY ?? "target/debug/cad-agent-studio.exe",
);
if (process.platform !== "win32" || !existsSync(binary)) {
  console.error(
    "NOT RUN — WINDOWS DESKTOP REQUIRED. Build desktop --debug --no-bundle, start tauri-driver on 127.0.0.1:4444, then run again.",
  );
  process.exit(1);
}
const base = "http://127.0.0.1:4444";
let session;
async function request(method, path, body) {
  const response = await fetch(base + path, {
    method,
    headers: { "content-type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(30000),
  });
  const data = await response.json();
  if (!response.ok || data.value?.error)
    throw new Error(JSON.stringify(data.value));
  return data.value;
}
try {
  session = (
    await request("POST", "/session", {
      capabilities: {
        alwaysMatch: { "tauri:options": { application: binary } },
      },
    })
  ).sessionId;
  await request("POST", `/session/${session}/timeouts`, { script: 20000 });
  const result = await request("POST", `/session/${session}/execute/async`, {
    script: `
  const done=arguments[arguments.length-1];
  (async()=>{
   const invoke=window.__TAURI_INTERNALS__.invoke;
   const initial=await invoke('runtime_status');
   if(initial.mode!=='live_unconnected'||initial.mcp!=='not_started') throw new Error('Unexpected automatic mock startup');
   await invoke('demo_enter');
   const health=await invoke('health');
   const project=await invoke('projects_create',{name:'Native test '+Date.now()});
   const conversation=await invoke('conversations_create',{projectId:project.id,title:'Native chat'});
   const run=await invoke('agent_start',{conversationId:conversation.id,content:'Vẽ tấm đế 200 x 120 mm, bốn lỗ đường kính 10 mm, tâm cách mép 20 mm.',mode:'execute'});
   let snapshot;
   const deadline=Date.now()+15000;
   do {
    snapshot=await invoke('agent_snapshot',{runId:run.id});
    if(snapshot.run.status!=='running') break;
    await new Promise(resolve=>setTimeout(resolve,100));
   } while(Date.now()<deadline);
   if(snapshot.run.status!=='awaiting_approval') throw new Error('Agent did not produce a proposal');
   const change=snapshot.change_sets[0];
   const before=await invoke('cad_get_context',{projectId:project.id});
   const receipt=await invoke('changes_approve',{changeId:change.id,payloadHash:change.payload_hash});
   const again=await invoke('changes_approve',{changeId:change.id,payloadHash:change.payload_hash});
   const after=await invoke('cad_get_context',{projectId:project.id});
   return {health,before,after,receipt,again};
  })().then(value=>done({ok:true,value}),error=>done({ok:false,error}));`,
    args: [],
  });
  assert.equal(result.ok, true, JSON.stringify(result.error));
  const r = result.value;
  assert.match(r.health, /Rust IPC/);
  assert.equal(r.before.geometry, null);
  assert.equal(r.after.geometry.entities.length, 8);
  assert.equal(r.after.revision, 1);
  assert.equal(r.receipt.operation_id, r.again.operation_id);
  console.log(
    "PASS — native WebView → Rust IPC → agent → MCP sidecar → approve/readback/idempotency",
  );
} finally {
  if (session) await request("DELETE", `/session/${session}`);
}
