# R1B acceptance — 07/10/2026

| Gate | Result | Evidence / limit |
|---|---|---|
| Browser account verification | LIVE_PASS, user-observed | User screenshot showed callback and verified ChatGPT profile. Earlier organization policy error was superseded. |
| Permission/model catalog | LIVE_PASS | Native runtime acquired 4 models using same ChatGPT profile; selected `gpt-5.6-luna`. `windows-live-runtime.json`. |
| Text inference | LIVE_PASS | Completed native runtime chat about a house 8 × 12 m, with CAD disconnected. `windows-live-runtime.json`. |
| Function calling | LIVE_PASS | Real model emitted `convert_length({value:1.25,from:m,to:mm})`; local result 1250 mm was sent back; model completed turn. `windows-live-runtime.json`. |
| Desktop UI → backend → model | LIVE_PASS | New native desktop displayed probe text/tool PASS in `windows-ui-inspect.json`; native chat completed in `windows-ui-chat-completed.json`. No Demo or CAD connection. |
| Persistence | NATIVE_PASS | Runtime drop/reopen retained 4 messages, account and selected model; native desktop after reopening loaded saved account/model and enabled conversation selector. `windows-live-runtime.json`, `windows-ui-restart-history.json`. |
| Secret boundary | NATIVE_PASS + bounded scan | DPAPI/WinCred tests; read-only scan of 5 SQLite tables found no API-key/JWT pattern. `windows-db-secret-pattern-check.json`; no claim of exhaustive memory/disk audit. |
| Regression/build | NATIVE_PASS | 53 Rust/vault tests including WinCred; 19 frontend tests, typecheck/lint/build; `windows-final-core.txt`, `windows-final-native-build.txt`, `windows-startup.json`. |

Earlier failed probes under this evidence directory were diagnostic runs before the
stream parser fix. The successful run and desktop UI evidence supersede them. A user
may still consume plan allowance when rerunning live checks. HTTP refresh/revoke
end-to-end and AutoCAD live bridge remain outside this acceptance.

Final artifact: `target/native-smoke/Chat R1B Final 92CF9ABF39E6/cad-agent-studio.exe`,
31,190,528 bytes, SHA256
`92CF9ABF39E6F10A21EABFE4057496E062D8FEA3C7E3E3715D739C4CA2796704`,
`r1b-1791345882`, PID 11776 at verification. Startup reported responding window,
zero mock sidecar children. Earlier instances were left untouched except the test
instance PID 13068, closed gracefully to release the account DB lock.
