# botc storyteller bridge for Windows PowerShell, in one line (the host's
# lobby shows it with the room's pairing code):
#
#   & ([scriptblock]::Create((irm https://botc.kiblitz.xyz/bridge.ps1))) <PAIRING-CODE>
#
# Fetches bridge.mjs from the same server and runs it with Node; any other
# arguments (--server, --codex-home, -- COMMAND...) go to it.
$server = 'https://botc.kiblitz.xyz'
for ($i = 0; $i -lt $args.Count - 1; $i++) {
    if ($args[$i] -eq '--server') { $server = $args[$i + 1] }
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Error 'botc-bridge: needs Node 22 or later (https://nodejs.org)'
    return
}
$script = Join-Path ([IO.Path]::GetTempPath()) ("botc-bridge-" + [Guid]::NewGuid() + ".mjs")
try {
    Invoke-WebRequest -UseBasicParsing "$server/bridge.mjs" -OutFile $script
    & node $script @args --server $server
} finally {
    Remove-Item -Force -ErrorAction SilentlyContinue $script
}
