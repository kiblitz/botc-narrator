# botc storyteller bridge for Windows PowerShell, in one line (the host's
# lobby shows it with the room's pairing code):
#
#   & ([scriptblock]::Create((irm https://botc.kiblitz.xyz/bridge.ps1))) <PAIRING-CODE>
#
# Fetches bridge.mjs from the same server and runs it with Node; any other
# arguments (--server, --codex-home, '--' COMMAND...) go to it. Quote '--':
# unquoted, PowerShell swallows it.
# The site, as in bridge.mjs and app.js's SITE.
$server = 'https://botc.kiblitz.xyz'
# Where to fetch bridge.mjs from: the user's --server, if any, before '--'.
for ($i = 0; $i -lt $args.Count - 1; $i++) {
    if ($args[$i] -eq '--') { break }
    if ($args[$i] -eq '--server') { $server = $args[$i + 1] }
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Error 'botc-bridge: needs Node 22 or later (https://nodejs.org)'
    return
}
$script = Join-Path ([IO.Path]::GetTempPath()) ("botc-bridge-" + [Guid]::NewGuid() + ".mjs")
try {
    Invoke-WebRequest -UseBasicParsing "$server/bridge.mjs" -OutFile $script
    # --server first: the user's own comes later and wins.
    & node $script --server $server @args
} finally {
    Remove-Item -Force -ErrorAction SilentlyContinue $script
}
