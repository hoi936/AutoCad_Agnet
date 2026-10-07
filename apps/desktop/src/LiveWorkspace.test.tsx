import {afterEach,beforeEach,expect,it,vi} from "vitest";
import {render,screen,fireEvent,waitFor,cleanup,act} from "@testing-library/react";
import {QueryClient,QueryClientProvider} from "@tanstack/react-query";
import {LiveWorkspace} from "./LiveWorkspace";
const api=vi.hoisted(()=>({accounts:vi.fn(),conversations:vi.fn(),messages:vi.fn(),create:vi.fn(),start:vi.fn(),snapshot:vi.fn(),login:vi.fn(),cancelLogin:vi.fn(),addKey:vi.fn(),catalog:vi.fn(),model:vi.fn(),signOut:vi.fn(),cancel:vi.fn()}));
vi.mock("./ipc",()=>({native:true,liveApi:api,errorMessage:(e:unknown)=>e instanceof Error?e.message:"Backend error"}));
const account={id:"account-a",auth_mode:"chatgpt",label:"ChatGPT A",status:"catalog_verified",selected_model:"model-a",models:[{id:"model-a",label:"Model A"}]};
const clients:QueryClient[]=[];
function mount(view="Chat"){const client=new QueryClient({defaultOptions:{queries:{retry:false}}});clients.push(client);return render(<QueryClientProvider client={client}><LiveWorkspace view={view} demo={false}/></QueryClientProvider>);}
beforeEach(()=>{vi.resetAllMocks();api.accounts.mockResolvedValue([account]);api.conversations.mockResolvedValue([]);api.messages.mockResolvedValue([]);api.create.mockResolvedValue("conversation-a");});
afterEach(()=>{cleanup();clients.splice(0).forEach(c=>c.clear());});
it("sends standalone chat while CAD is disconnected without a project or demo",async()=>{
 const run={id:"run-a",conversation_id:"conversation-a",account_id:"account-a",model:"model-a",status:"completed",text:"Cần số tầng và phòng.",events:[],trace:[],error_code:null};api.start.mockResolvedValue(run);api.snapshot.mockResolvedValue(run);
 mount();await waitFor(()=>expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeEnabled());fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"),{target:{value:"Nhà 8x12 cần thông tin gì?"}});fireEvent.click(screen.getByRole("button",{name:"Gửi chat"}));
 await waitFor(()=>expect(api.start).toHaveBeenCalledExactlyOnceWith("conversation-a","Nhà 8x12 cần thông tin gì?",false));expect(screen.getByText(/CAD: chưa kết nối/)).toBeInTheDocument();
});
it("keeps an active run pinned and requests cancellation without switching account",async()=>{
 const run={id:"run-a",conversation_id:"conversation-a",account_id:"account-a",model:"model-a",status:"running",text:"",events:[],trace:[],error_code:null};api.start.mockResolvedValue(run);api.snapshot.mockResolvedValue(run);api.cancel.mockResolvedValue(undefined);
 mount();await waitFor(()=>expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeEnabled());fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"),{target:{value:"hello"}});fireEvent.click(screen.getByRole("button",{name:"Gửi chat"}));
 fireEvent.click(await screen.findByRole("button",{name:"Dừng lượt chat"}));expect(screen.getByLabelText("Profile LLM")).toBeDisabled();await waitFor(()=>expect(api.cancel).toHaveBeenCalledWith("run-a"));
});
it("clears a one-shot global API key even when the native vault fails",async()=>{
 api.accounts.mockResolvedValue([]);api.addKey.mockRejectedValue(new Error("Vault failed"));mount("Kết nối");fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(screen.getByText("OpenAI API key — lựa chọn riêng"));fireEvent.change(screen.getByLabelText("API key độc lập"),{target:{value:"fixture-key"}});fireEvent.click(screen.getByRole("button",{name:"Lưu API key"}));
 expect(screen.getByLabelText("API key độc lập")).toHaveValue("");expect(await screen.findByRole("alert")).toHaveTextContent("Vault failed");expect(api.addKey).toHaveBeenCalledWith("OpenAI API","fixture-key",true);
});
it("does not replace the selected account before a pending login is validated",async()=>{
 let reject:(reason:Error)=>void=()=>{};api.login.mockImplementation(()=>new Promise((_,r)=>{reject=r;}));mount("Kết nối");await screen.findByText(/ChatGPT A · Gói ChatGPT/);fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(screen.getByRole("button",{name:"Continue with ChatGPT"}));
 expect(screen.getByLabelText("Profile LLM")).toHaveValue("account-a");expect(await screen.findByRole("button",{name:"Hủy đăng nhập"})).toBeEnabled();await act(async()=>reject(new Error("Identity mismatch")));expect(await screen.findByRole("alert")).toHaveTextContent("Identity mismatch");expect(screen.getByLabelText("Profile LLM")).toHaveValue("account-a");
});

it("loads the verified account catalog after login without starting inference",async()=>{
 api.login.mockResolvedValue(account);api.catalog.mockResolvedValue(account);mount("Kết nối");
 await screen.findByText(/ChatGPT A · Gói ChatGPT/);fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(screen.getByRole("button",{name:"Continue with ChatGPT"}));
 await screen.findByText("Đã lấy danh sách model. Chọn model rồi bấm Kiểm tra LLM.");expect(api.catalog).toHaveBeenCalledExactlyOnceWith("account-a");expect(api.start).not.toHaveBeenCalled();
});
it("preserves successful identity verification when catalog retrieval fails",async()=>{
 api.login.mockResolvedValue(account);api.catalog.mockRejectedValue(new Error("Catalog unavailable"));mount("Kết nối");
 await screen.findByText(/ChatGPT A · Gói ChatGPT/);fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(screen.getByRole("button",{name:"Continue with ChatGPT"}));
 expect(await screen.findByRole("alert")).toHaveTextContent("Catalog unavailable");expect(screen.getByText(/Đã xác minh tài khoản; chưa lấy được/)).toBeInTheDocument();expect(screen.getByLabelText("Profile LLM")).toHaveValue("account-a");expect(api.start).not.toHaveBeenCalled();
});

it("shows a completed answer once after messages refresh",async()=>{
 const finished={id:"run-finished",conversation_id:"conversation-a",account_id:"account-a",model:"model-a",status:"completed",text:"Cần số tầng.",events:[],trace:[],error_code:null};
 api.start.mockResolvedValue(finished);api.snapshot.mockResolvedValue(finished);
 api.messages.mockResolvedValue([{run_id:"run-finished",role:"assistant",content:"Cần số tầng."}]);
 mount();await waitFor(()=>expect(screen.getByLabelText("Yêu cầu thiết kế")).toBeEnabled());
 fireEvent.change(screen.getByLabelText("Yêu cầu thiết kế"),{target:{value:"Nhà cần gì?"}});fireEvent.click(screen.getByRole("button",{name:"Gửi chat"}));
 await waitFor(()=>expect(api.start).toHaveBeenCalled());await screen.findByText("Cần số tầng.");
 expect(screen.getAllByText("Cần số tầng.")).toHaveLength(1);
});
