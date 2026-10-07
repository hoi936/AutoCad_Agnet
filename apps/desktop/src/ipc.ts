import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  Project,
  DocumentContext,
  ChangeSet,
  ExecutionReceipt,
  PlateParameters,
  RunMode,
  Conversation,
  ConversationSnapshot,
  AgentRun,
  RunSnapshot,
  ProviderConfig,
} from "./contracts";
export const native = isTauri();
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!native)
    return Promise.reject(
      new Error(
        "Chế độ trình duyệt chỉ xem giao diện. Hãy chạy pnpm dev để kết nối Rust.",
      ),
    );
  return invoke<T>(command, args);
}
export type RuntimeStatus = { build_id: string; runtime: string; mode: "live_unconnected" | "live_read_only" | "demo"; cad: string; mcp: string; oauth: string };
export const api = {
  runtime: () => call<RuntimeStatus>("runtime_status"),
  enterDemo: () => call<void>("demo_enter"),
  cadCandidates: () => call<number[]>("cad_candidates"),
  cadPair: (pid:number, code:string) => call<{connected:boolean;document:{file_name:string;units:string}}>("cad_pair",{pid,code}),
  cadLiveStatus: () => call<{connected:boolean;active_document_id?:string}>("cad_live_status"),
  cadDisconnect: () => call<void>("cad_disconnect"),
  health: () => call<string>("health"),
  projects: () => call<Project[]>("projects_list"),
  create: (name: string) => call<Project>("projects_create", { name }),
  context: (projectId: string) =>
    call<DocumentContext>("cad_get_context", { projectId }),
  changes: (projectId: string) =>
    call<ChangeSet[]>("changes_list", { projectId }),
  propose: (projectId: string, parameters: PlateParameters, mode: RunMode) =>
    call<ChangeSet>("design_propose_plate", { projectId, parameters, mode }),
  approve: (change: ChangeSet) =>
    call<ExecutionReceipt>("changes_approve", {
      changeId: change.id,
      payloadHash: change.payload_hash,
    }),
  recover: (changeId: string) => call<ExecutionReceipt>("changes_recover", { changeId }),
  reject: (changeId: string) => call<void>("changes_reject", { changeId }),
  revert: (changeId: string) => call<ChangeSet>("changes_revert", { changeId }),
  conversations: (projectId: string) =>
    call<Conversation[]>("conversations_list", { projectId }),
  createConversation: (projectId: string, title: string) =>
    call<Conversation>("conversations_create", { projectId, title }),
  conversation: (conversationId: string) =>
    call<ConversationSnapshot>("conversation_snapshot", { conversationId }),
  startRun: (conversationId: string, content: string, mode: RunMode) =>
    call<AgentRun>("agent_start", { conversationId, content, mode }),
  run: (runId: string) => call<RunSnapshot>("agent_snapshot", { runId }),
  stopRun: (runId: string) => call<void>("agent_stop", { runId }),
  provider: (projectId: string) =>
    call<ProviderConfig>("provider_get_config", { projectId }),
  saveProvider: (
    projectId: string,
    provider: string,
    model: string,
    cloudConsent: boolean,
    storage: string,
  ) => call<ProviderConfig>("provider_save_config", {
    projectId, provider, model, cloudConsent, storage,
  }),
  setSecret: (projectId: string, secret: string) =>
    call<void>("provider_set_secret", { projectId, secret }),
  deleteSecret: (projectId: string) =>
    call<void>("provider_delete_secret", { projectId }),
};
export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "Thao tác thất bại. Vui lòng kiểm tra kết nối.";
}

export interface LlmAccount { id:string; auth_mode:"api_key"|"chatgpt"; label:string; issuer:string|null; subject:string|null; client_id:string|null; scope:string; expires_at:number|null; selected_model:string|null; status:string; models:{id:string;label:string}[] }
export interface StandaloneRun {id:string;conversation_id:string;account_id:string;model:string;status:string;text:string;error_code:string|null;trace:unknown[];events:{run_id:string;sequence:number;kind:string;text:string}[]}
export const liveApi={
 accounts:()=>call<LlmAccount[]>("account_list"),
 addKey:(label:string,secret:string,consent:boolean)=>call<LlmAccount>("account_add_key",{label,secret,consent}),
 login:(accountId:string|null,consent:boolean)=>call<LlmAccount>("account_sign_in",{accountId,consent}),
 cancelLogin:()=>call<void>("account_cancel_login"),
 signOut:(accountId:string)=>call<string>("account_sign_out",{accountId}),
 catalog:(accountId:string)=>call<LlmAccount>("model_catalog",{accountId}),
 model:(accountId:string,model:string)=>call<LlmAccount>("account_select_model",{accountId,model}),
 conversations:(accountId:string)=>call<{id:string;title:string}[]>("standalone_conversations",{accountId}),
 create:(accountId:string)=>call<string>("standalone_create",{accountId}),
 messages:(conversationId:string)=>call<{role:string;content:string;run_id:string}[]>("standalone_messages",{conversationId}),
 start:(conversationId:string,content:string,probe=false)=>call<StandaloneRun>("standalone_start",{conversationId,content,probe}),
 snapshot:(runId:string)=>call<StandaloneRun>("standalone_snapshot",{runId}),
 cancel:(runId:string)=>call<void>("standalone_cancel",{runId}),
};
