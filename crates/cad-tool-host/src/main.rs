#![forbid(unsafe_code)]
use rmcp::ServiceExt;
#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--live-read-stdio"] {
        match cad_tool_host::live::LiveServer::new().serve((
            cad_tool_host::bounded::BoundedRead::new(tokio::io::stdin()),
            tokio::io::stdout(),
        )).await {
            Ok(service) => { let _ = service.waiting().await; }
            Err(_) => { eprintln!("CAD live MCP initialization failed"); std::process::exit(1); }
        }
        return;
    }
    let paired = args == ["--paired-stdio"];
    let server = cad_tool_host::server::CadServer::new(paired);
    match server
        .serve((
            cad_tool_host::bounded::BoundedRead::new(tokio::io::stdin()),
            tokio::io::stdout(),
        ))
        .await
    {
        Ok(service) => {
            let _ = service.waiting().await;
        }
        Err(_) => {
            eprintln!("CAD MCP initialization failed");
            std::process::exit(1);
        }
    }
}
