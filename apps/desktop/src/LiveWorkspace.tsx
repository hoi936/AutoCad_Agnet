import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { errorMessage, liveApi, native, type StandaloneRun } from "./ipc";
export function LiveWorkspace({ view, demo }: {view:string;demo:boolean}) {
 const client=useQueryClient();
 const [selected,setSelected]=useState(""); const [conversation,setConversation]=useState("");
 const [content,setContent]=useState(""); const [label,setLabel]=useState("OpenAI API");const [secret,setSecret]=useState("");
 const [consent,setConsent]=useState(false);const [busy,setBusy]=useState(false);const [loggingIn,setLoggingIn]=useState(false);
 const [error,setError]=useState("");const [notice,setNotice]=useState("");const [current,setCurrent]=useState<StandaloneRun|null>(null);
 const [probe,setProbe]=useState(false);const observed=useRef("");
 const accounts=useQuery({queryKey:["accounts"],queryFn:liveApi.accounts,enabled:native,retry:false});
 const account=accounts.data?.find(a=>a.id===selected) ?? accounts.data?.[0];
 const accountId=account?.id??"";
 const conversations=useQuery({queryKey:["live-conversations",accountId],queryFn:()=>liveApi.conversations(accountId),enabled:native&&!!accountId,retry:false});
 const messages=useQuery({queryKey:["live-messages",conversation],queryFn:()=>liveApi.messages(conversation),enabled:native&&!!conversation,retry:false});
 const run=useQuery({queryKey:["live-run",current?.id],queryFn:()=>liveApi.snapshot(current!.id),enabled:native&&!!current,retry:false,refetchInterval:q=>q.state.data?.status==="running"||(!q.state.data&&current?.status==="running")?300:false});
 const snapshot=run.data?.id===current?.id&&run.data?.account_id===accountId?run.data:current;
 const running=snapshot?.status==="running";const locked=busy||running||loggingIn;
 const ready=!!account?.selected_model&&!['signed_out','missing_permission','expired'].includes(account.status);
 const events=snapshot?.events.filter(e=>e.run_id===snapshot.id)??[];
 const stream=events.filter(e=>e.kind==="text.delta").map(e=>e.text).join("");
 useEffect(()=>{
  if(snapshot&&snapshot.status!=="running"&&observed.current!==snapshot.id){observed.current=snapshot.id;void client.invalidateQueries({queryKey:["live-messages",snapshot.conversation_id]});void client.invalidateQueries({queryKey:["accounts"]});}
 },[snapshot,client]);
 async function action(fn:()=>Promise<void>){setBusy(true);setError("");setNotice("");try{await fn();await client.invalidateQueries({queryKey:["accounts"]});}catch(e){setError(errorMessage(e));}finally{await client.invalidateQueries({queryKey:["accounts"]});setBusy(false);setSecret("");}}
 async function refreshCatalog(id:string){
  const result=await liveApi.catalog(id);
  setNotice(result.models.length?"Đã lấy danh sách model. Chọn model rồi bấm Kiểm tra LLM.":"Tài khoản chưa có model khả dụng trong catalog. Kiểm tra quyền của tài khoản rồi làm mới.");
 }
 async function login(returning=false){
  setLoggingIn(true);setError("");setNotice("");
  try{
   const result=await liveApi.login(returning?accountId:null,consent);
   setSelected(result.id);setConversation("");setCurrent(null);
   if(result.status==="missing_permission")setNotice("Danh tính đã xác minh nhưng chưa có quyền inference.");
   else{
    setNotice("Đã xác minh tài khoản. Đang lấy danh sách model…");
    try{await refreshCatalog(result.id);}catch(e){setNotice("Đã xác minh tài khoản; chưa lấy được danh sách model. Bấm Refresh models để thử lại.");setError(errorMessage(e));}
   }
  }catch(e){setError(errorMessage(e));}
  finally{await client.invalidateQueries({queryKey:["accounts"]});setLoggingIn(false);}
 }
 async function send(asProbe=false){if(!account||!ready||locked)return;setBusy(true);setError("");setProbe(asProbe);try{let target=conversation;if(!target){target=await liveApi.create(account.id);setConversation(target);await client.invalidateQueries({queryKey:["live-conversations",account.id]});}const started=await liveApi.start(target,content,asProbe);setCurrent(started);setContent("");await client.invalidateQueries({queryKey:["live-messages",target]});}catch(e){setError(errorMessage(e));}finally{setBusy(false);}}
 const statuses:Record<string,string>={stored_unverified:"Đã lưu khóa; chưa xác thực",verified:"Đã xác minh; chưa thử inference",missing_permission:"Thiếu quyền inference",expired:"Hết hạn / refresh thất bại",signed_out:"Đã đăng xuất",catalog_verified:"Đã lấy catalog; inference cần kiểm thử",inference_verified:"Inference đã chạy thành công",catalog_error:"Lỗi catalog"};
 return <section className="connection-page" hidden={view!=="Kết nối"&&(view!=="Chat"||demo)} aria-label="LLM độc lập">
  <h1>{view==="Chat"?"Bạn muốn thiết kế gì?":"Tài khoản & mô hình"}</h1>
  <p>LLM: {account?statuses[account.status]??account.status:"Chưa kết nối"} · CAD: chưa kết nối · Không cần project hoặc Demo.</p>
  <label>Profile <select aria-label="Profile LLM" disabled={locked||!accounts.data?.length} value={accountId} onChange={e=>{setSelected(e.target.value);setConversation("");setCurrent(null);setError("");}}>
   {!accounts.data?.length&&<option value="">Chưa có profile</option>}{accounts.data?.map(a=><option key={a.id} value={a.id}>{a.label} · {a.auth_mode}</option>)}
  </select></label>
  {account&&<p>{account.label} · {account.auth_mode==="chatgpt"?"Gói ChatGPT":"OpenAI API — billing riêng"} · {account.selected_model??"Chưa chọn model"}</p>}
  {view==="Kết nối"&&<>
   <label className="checkbox"><input type="checkbox" checked={consent} disabled={locked} onChange={e=>setConsent(e.target.checked)}/>Tôi đồng ý gửi nội dung chat và kết quả tool tới OpenAI. Live inference/probe có thể dùng hạn mức hoặc phát sinh phí theo route đã chọn.</label>
   <div className="actions"><button disabled={!native||locked||!consent} onClick={()=>void login()}>Continue with ChatGPT</button>
   {account?.auth_mode==="chatgpt"&&<button disabled={!native||locked||!consent} onClick={()=>void login(true)}>Đăng nhập lại profile này</button>}
   {loggingIn&&<button onClick={()=>void liveApi.cancelLogin().catch(e=>setError(errorMessage(e)))}>Hủy đăng nhập</button>}</div>
   {loggingIn&&<p role="status">Đang chờ đăng nhập trong browser hệ thống và xác minh tài khoản…</p>}
   <details><summary>OpenAI API key — lựa chọn riêng</summary><form onSubmit={e=>{e.preventDefault();const value=secret;setSecret("");void action(async()=>{const a=await liveApi.addKey(label,value,consent);setSelected(a.id);setConversation("");setCurrent(null);setNotice("Đã lưu API key vào Windows Credential Manager. Làm mới model để xác thực.");});}}>
    <label>Nhãn profile<input value={label} maxLength={120} disabled={locked} onChange={e=>setLabel(e.target.value)}/></label>
    <label>API key<input aria-label="API key độc lập" type="password" autoComplete="off" value={secret} disabled={!native||locked} onChange={e=>setSecret(e.target.value)}/></label>
    <button disabled={!native||locked||!secret||!consent}>Lưu API key</button>
   </form></details>
   {account&&<><div className="actions"><button disabled={!native||locked} onClick={()=>void action(async()=>{await refreshCatalog(account.id);})}>Refresh models</button>
   <button disabled={!native||locked} onClick={()=>void action(async()=>setNotice(await liveApi.signOut(account.id)))}>Sign out</button></div>
   <label>Model <select aria-label="Model LLM" value={account.selected_model??""} disabled={locked||!account.models.length} onChange={e=>void action(async()=>{await liveApi.model(account.id,e.target.value);})}>
    <option value="" disabled>Chọn model từ catalog của profile</option>{account.models.map(m=><option value={m.id} key={m.id}>{m.label}</option>)}
   </select></label>
   <p>Catalog không chứng minh model hỗ trợ text/tool. Nút kiểm tra sẽ gọi inference thật và có thể dùng hạn mức/chi phí.</p>
   <button disabled={!native||locked||!ready} onClick={()=>void send(true)}>Kiểm tra LLM</button></>}
  </>}
  {view==="Chat"&&<>
   <div className="actions"><select aria-label="Hội thoại live" value={conversation} disabled={locked||!conversations.data?.length} onChange={e=>{setConversation(e.target.value);setCurrent(null);}}><option value="">Chọn hội thoại</option>{conversations.data?.map(c=><option key={c.id} value={c.id}>{c.title} · {c.id.slice(0,8)}</option>)}</select>
   <button disabled={!native||locked||!accountId} onClick={()=>void action(async()=>{const id=await liveApi.create(accountId);setConversation(id);setCurrent(null);await client.invalidateQueries({queryKey:["live-conversations",accountId]});})}>Hội thoại mới</button></div>
   <div className="messages" aria-live="polite">{messages.data?.map((m,i)=><article className={`message ${m.role}`} key={`${m.run_id}:${i}`}><strong>{m.role==="user"?"Bạn":"Agent"}</strong><p>{m.content}</p></article>)}
   {running&&stream&&<article className="message assistant"><p>{stream}</p></article>}</div>
   <form onSubmit={e=>{e.preventDefault();void send();}}><label htmlFor="live-brief">Yêu cầu thiết kế</label><textarea id="live-brief" value={content} onChange={e=>setContent(e.target.value)} rows={4} maxLength={16000} disabled={!native||locked||!ready} placeholder="Tôi muốn thiết kế sơ bộ nhà 8 × 12 m thì cần cung cấp thông tin nào?"/>
   <button disabled={!native||locked||!ready||!content.trim()}>Gửi chat</button></form>
   {!ready&&<p>Mở Kết nối, đăng nhập hoặc thêm API key, lấy catalog và chọn model để chat thật. Không tự chuyển sang Demo.</p>}
  </>}
  {running&&<button onClick={()=>void liveApi.cancel(snapshot!.id).catch(e=>setError(errorMessage(e)))}>Dừng lượt chat</button>}
  {snapshot&&<div><p role="status">Run {snapshot.id} · {snapshot.status}{snapshot.error_code?` · ${snapshot.error_code}`:""}</p>
   {snapshot.status!=="running"&&snapshot.text&&!(snapshot.status==="completed"&&messages.data?.some(m=>m.run_id===snapshot.id&&m.role==="assistant"))&&<p>{snapshot.text}</p>}
   {(snapshot.trace.length>0||events.some(e=>e.kind==="tool.completed"))&&<details><summary>Local tool trace</summary><pre>{JSON.stringify(snapshot.trace.length?snapshot.trace:events.filter(e=>e.kind==="tool.completed").map(e=>JSON.parse(e.text)),null,2)}</pre></details>}
   {probe&&<p>Text inference: {snapshot.status==="completed"?"PASS":"chưa đạt"} · Function calling: {snapshot.status==="completed"&&snapshot.trace.length>0?"PASS — output được gửi lại model, turn hoàn tất":"chưa đạt"}</p>}
  </div>}
  {(error||accounts.error||messages.error||run.error)&&<p role="alert">{error||errorMessage(accounts.error||messages.error||run.error)}</p>}{notice&&<p role="status">{notice}</p>}
 </section>;
}
