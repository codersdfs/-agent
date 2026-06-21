@echo off
rem Run the Omega Agent CLI from the repo root.
setlocal enabledelayedexpansion
set ROOT=%~dp0
pushd "%ROOT%src-tauri"
cargo run --manifest-path "%ROOT%src-tauri\Cargo.toml" -- --cli %*
set EXITCODE=%ERRORLEVEL%
popd
endlocal
exit /b %EXITCODE%
