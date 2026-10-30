# Hoard installer - Windows (PowerShell)
#
#   irm https://hoard.services/install.ps1 | iex
#
# Downloads the core for this machine (hoardd.exe, the sync engine, and
# hoard.exe, the terminal), checks its SHA-256 and hands over to `hoard install`,
# which does everything else: puts the core in %LOCALAPPDATA%\hoard\bin, takes
# over from any older Hoard on this machine, starts the sync service and
# installs the app. That logic lives in Rust so this script, Hoard Setup and the
# app's own updater can't drift apart; this file only fetches and checks.
#
# Override with environment variables before running:
#   $env:HOARD_VERSION      = '1.2.2'    # pin a version instead of the latest
#   $env:HOARD_INSTALL_DIR  = 'C:\tools' # put the core somewhere else
#   $env:HOARD_HEADLESS     = '1'        # core only, never the desktop app

$ErrorActionPreference = 'Stop'
# Invoke-WebRequest's progress bar makes large downloads crawl on Windows
# PowerShell 5.1.
$ProgressPreference = 'SilentlyContinue'
$Releases = 'https://github.com/rleeon/hoard/releases'

# Windows PowerShell 5.1 defaults to TLS 1.0/1.1, which GitHub rejects.
try {
  [Net.ServicePointManager]::SecurityProtocol =
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
} catch {}

function Info($m) { Write-Host "==> $m" -ForegroundColor Green }
function Die($m)  { Write-Host "error: $m" -ForegroundColor Red; exit 1 }

# tar.exe ships with Windows 10 (1803+) / 11.
if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) {
  Die "tar.exe not found. Windows 10 1803+ or Windows 11 is required."
}

# The OS variables rather than RuntimeInformation, which comes back empty in
# some 5.1 hosts. PROCESSOR_ARCHITEW6432 holds the real architecture when an
# emulated shell runs on ARM64.
$rawArch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
switch ("$rawArch".ToUpper()) {
  'AMD64' { $arch = 'x86_64' }
  'X64'   { $arch = 'x86_64' }
  'ARM64' { $arch = 'aarch64' }
  default { Die "unsupported architecture '$rawArch'." }
}

# The latest version is read off where `releases/latest` redirects, not from
# GitHub's API, which allows 60 requests an hour per address.
$ver = $env:HOARD_VERSION
if ([string]::IsNullOrWhiteSpace($ver)) {
  try {
    $req = [System.Net.HttpWebRequest]::Create("$Releases/latest")
    $req.Method = 'HEAD'
    $req.UserAgent = 'hoard-installer'
    $resp = $req.GetResponse()
    $ver = ($resp.ResponseUri.AbsolutePath -split '/')[-1]
    $resp.Close()
  } catch {
    Die "couldn't find out the latest version. Set `$env:HOARD_VERSION and retry."
  }
}
$ver = $ver.TrimStart('v')
if ($ver -notmatch '^\d+\.\d+\.\d+') { Die "couldn't find out the latest version (got '$ver')." }

$asset = "hoard-$ver-windows-$arch.tar.gz"
$url   = "$Releases/download/v$ver/$asset"
$tmp = Join-Path $env:TEMP ("hoard-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
try {
  $pkg = Join-Path $tmp 'core.tar.gz'
  $sha = Join-Path $tmp 'core.sha256'
  Info "Downloading $asset"
  try { Invoke-WebRequest $url -OutFile $pkg -UseBasicParsing } catch { Die "download failed: $url" }
  # To a file and read back as text: on 5.1 the `.Content` of a non-text
  # response is a byte[], and splitting that yields the first byte's value.
  try { Invoke-WebRequest "$url.sha256" -OutFile $sha -UseBasicParsing } catch {
    Die "no checksum published for $asset; not installing it."
  }
  $expected = ((Get-Content $sha -Raw).Trim() -split '\s+')[0].ToLower()
  $actual = (Get-FileHash $pkg -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { Die "checksum mismatch for $asset. Not installing it." }

  tar.exe -xzf $pkg -C $tmp
  if ($LASTEXITCODE -ne 0) { Die "failed to extract the archive." }
  $bin = Join-Path $tmp "hoard-$ver-windows-$arch\hoard.exe"
  if (-not (Test-Path $bin)) { Die "the archive has no hoard.exe." }

  $restArgs = @('install')
  if ($env:HOARD_HEADLESS -eq '1') { $restArgs += '--headless' }

  # Up to 1.2.1, `hoard install` expected to be run from where it lives already,
  # so a pinned older version gets put in place first, the way this script used
  # to do it.
  if ([version]$ver -lt [version]'1.2.2') {
    $dir = $env:HOARD_INSTALL_DIR
    if ([string]::IsNullOrWhiteSpace($dir)) { $dir = Join-Path $env:LOCALAPPDATA 'hoard\bin' }
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Get-Process hoardd, hoard-desktop -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    foreach ($b in @('hoard.exe', 'hoardd.exe')) {
      Copy-Item (Join-Path (Split-Path $bin) $b) (Join-Path $dir $b) -Force
    }
    $bin = Join-Path $dir 'hoard.exe'
  }

  Info "Installing Hoard $ver"
  $env:HOARD_NONINTERACTIVE = '1'
  & $bin @restArgs
  if ($LASTEXITCODE -ne 0) { Die "the install didn't finish; run it again, or use Hoard Setup." }
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Info "Done. Next: hoard login  (open a new terminal first)"
