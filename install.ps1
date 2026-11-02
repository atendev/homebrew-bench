# Install or update `bench`, its legal notices, and the homebrew-bench agent skill.
#
#   irm https://raw.githubusercontent.com/atendev/homebrew-bench/master/install.ps1 | iex
#   ./install.ps1 [--no-skills] [--agent <name>]... [--version <tag>] [--from-source]
#
# Environment overrides: BENCH_HOME (default %USERPROFILE%\.bench), BENCH_REPO
# (owner/name), BENCH_RELEASES_URL (release download base), BENCH_GIT_URL
# (source to clone), BENCH_TARGET (target triple).

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Repo = if ($env:BENCH_REPO) { $env:BENCH_REPO } else { 'atendev/homebrew-bench' }
$Releases = if ($env:BENCH_RELEASES_URL) { $env:BENCH_RELEASES_URL } else { "https://github.com/$Repo/releases" }
$GitUrl = if ($env:BENCH_GIT_URL) { $env:BENCH_GIT_URL } else { "https://github.com/$Repo.git" }
$Skill = 'homebrew-bench'

function Show-Usage {
    @'
Usage: install.ps1 [options]

Installs bench into %USERPROFILE%\.bench\bin, with NOTICE, LICENSE and the
homebrew-bench skill, then copies the skill into each agent's user skill
folder. Re-running updates the install and leaves config.toml, index,
parties\, house-rules\, encounters\ and reports\ alone.

Options:
  --no-skills          Install no agent skills outside .bench.
  --agent <name>       Install the skill for this agent only: claude, cline,
                       kiro, agents (~\.agents\skills) or all. Repeatable.
                       Without it, the skill goes to ~\.agents\skills plus
                       each of claude, cline and kiro found on PATH or with a
                       ~\.<agent> folder.
  --version <tag>      Release to install (default: latest).
  --from-source        Build with cargo instead of downloading a release.
                       Inside a checkout this builds the checkout, which is
                       also the default when --version is not given.
  -h, --help           Show this help.
'@
}

function Fail([string]$Message) {
    [Console]::Error.WriteLine("install.ps1: $Message")
    exit 1
}

function Fail-Usage([string]$Message) {
    [Console]::Error.WriteLine("install.ps1: $Message")
    [Console]::Error.WriteLine((Show-Usage))
    exit 2
}

$NoSkills = $false
$Agents = [System.Collections.Generic.List[string]]::new()
$Version = ''
$FromSource = $false

function Add-Agent([string]$Name) {
    switch ($Name) {
        { $_ -in 'claude', 'cline', 'kiro', 'agents' } { $Agents.Add($Name); return }
        'all' { 'agents', 'claude', 'cline', 'kiro' | ForEach-Object { $Agents.Add($_) }; return }
        default { Fail-Usage "unknown agent '$Name': use claude, cline, kiro, agents or all" }
    }
}

for ($i = 0; $i -lt $args.Count; $i++) {
    $arg = [string]$args[$i]
    switch -Regex ($arg) {
        '^--no-skills$' { $NoSkills = $true; continue }
        '^--agent$' {
            if ($i + 1 -ge $args.Count) { Fail-Usage '--agent needs a value' }
            $i++; Add-Agent ([string]$args[$i]); continue
        }
        '^--agent=' { Add-Agent $arg.Substring(8); continue }
        '^--version$' {
            if ($i + 1 -ge $args.Count) { Fail-Usage '--version needs a value' }
            $i++; $Version = [string]$args[$i]; continue
        }
        '^--version=' { $Version = $arg.Substring(10); continue }
        '^--from-source$' { $FromSource = $true; continue }
        '^(-h|--help)$' { Show-Usage; exit 0 }
        default { Fail-Usage "unknown option '$arg' (see --help)" }
    }
}

if ($NoSkills -and $Agents.Count -gt 0) { Fail-Usage '--no-skills and --agent cannot be combined' }

$UserHome = if ($env:USERPROFILE) { $env:USERPROFILE } else { $HOME }
$BenchHome = if ($env:BENCH_HOME) { $env:BENCH_HOME } else { Join-Path $UserHome '.bench' }

$Checkout = ''
if ($PSScriptRoot -and (Test-Path (Join-Path $PSScriptRoot 'bench\Cargo.toml')) -and
    (Test-Path (Join-Path $PSScriptRoot ".agents\skills\$Skill\SKILL.md"))) {
    $Checkout = $PSScriptRoot
}
if ($FromSource -and $Checkout -and $Version) {
    Fail-Usage '--version picks a release, but --from-source inside a checkout builds the checkout; drop one'
}

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("bench-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $Tmp | Out-Null

$StageBin = ''
$StageDir = ''
$StageSkill = ''

function Find-SkillDir([string]$Root) {
    foreach ($candidate in (Join-Path $Root $Skill), (Join-Path $Root "skills\$Skill"), (Join-Path $Root ".agents\skills\$Skill")) {
        if (Test-Path (Join-Path $candidate 'SKILL.md')) { return $candidate }
    }
    return $null
}

function Get-Target {
    if ($env:BENCH_TARGET) { return $env:BENCH_TARGET }
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    # Windows on ARM runs the x64 build under emulation.
    if ($arch -in 'X64', 'Arm64') { return 'x86_64-pc-windows-msvc' }
    return $null
}

function Invoke-StageRelease {
    $target = Get-Target
    if (-not $target) {
        Write-Warning 'no prebuilt bench for this architecture'
        return $false
    }
    $archive = "bench-$target.zip"
    $url = if (-not $Version -or $Version -eq 'latest') { "$Releases/latest/download/$archive" } else { "$Releases/download/$Version/$archive" }
    $path = Join-Path $Tmp $archive
    Write-Host "Downloading $url"
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $path
    } catch {
        Write-Warning "no release archive at $url"
        return $false
    }
    $haveChecksum = $true
    try {
        Invoke-WebRequest -UseBasicParsing -Uri "$url.sha256" -OutFile "$path.sha256"
    } catch {
        $haveChecksum = $false
    }
    if ($haveChecksum) {
        $want = ((Get-Content "$path.sha256" -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
        $have = (Get-FileHash -Algorithm SHA256 $path).Hash.ToLowerInvariant()
        if ($want -ne $have) { Fail "checksum mismatch for ${archive}: expected $want, got $have" }
    }
    $out = Join-Path $Tmp 'release'
    Expand-Archive -Path $path -DestinationPath $out -Force
    $bin = Get-ChildItem -Path $out -Recurse -Depth 2 -File -Filter 'bench.exe' | Select-Object -First 1
    if (-not $bin) { Fail "$archive has no bench.exe" }
    $script:StageBin = $bin.FullName
    $script:StageDir = $bin.DirectoryName
    $script:StageSkill = Find-SkillDir $bin.DirectoryName
    if (-not $script:StageSkill) { Fail "$archive has no $Skill skill" }
    return $true
}

function Invoke-StageSource {
    $src = $Checkout
    if (-not $src) {
        if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Fail 'need git to fetch the source' }
        $src = Join-Path $Tmp 'src'
        Write-Host "Cloning $GitUrl"
        $cloneArgs = @('clone', '--quiet', '--depth', '1')
        if ($Version -and $Version -ne 'latest') { $cloneArgs += @('--branch', $Version) }
        & git @cloneArgs $GitUrl $src
        if ($LASTEXITCODE -ne 0) { Fail "could not clone $GitUrl" }
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Fail 'need cargo to build from source; install Rust from https://rustup.rs'
    }
    Write-Host "Building bench in $src"
    Push-Location $src
    try {
        & cargo build --release --locked -p bench
        if ($LASTEXITCODE -ne 0) { Fail 'cargo build failed' }
    } finally {
        Pop-Location
    }
    $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    if (-not [System.IO.Path]::IsPathRooted($targetDir)) { $targetDir = Join-Path $src $targetDir }
    $bin = Join-Path $targetDir 'release\bench.exe'
    if (-not (Test-Path $bin)) { Fail "cargo build did not produce $bin" }
    $script:StageBin = $bin
    $script:StageDir = $src
    # Copy from the canonical folder, never the .claude\skills symlink, which
    # a Windows checkout without symlink support holds as a plain text file.
    $script:StageSkill = Join-Path $src ".agents\skills\$Skill"
}

function Copy-Dir([string]$From, [string]$To) {
    $staging = "$To.new"
    if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
    New-Item -ItemType Directory -Force -Path (Split-Path $To -Parent) | Out-Null
    Copy-Item -Recurse -Path $From -Destination $staging
    if (Test-Path $To) { Remove-Item -Recurse -Force $To }
    Move-Item -Path $staging -Destination $To
}

try {
    if ($FromSource -or ($Checkout -and -not $Version)) {
        Invoke-StageSource
    } elseif (-not (Invoke-StageRelease)) {
        # A release was asked for, so build that release's source, not the checkout.
        $label = if ($Version) { $Version } else { 'latest' }
        Write-Warning "building $label from source instead"
        $script:Checkout = ''
        Invoke-StageSource
    }

    foreach ($file in 'NOTICE', 'LICENSE') {
        if (-not (Test-Path (Join-Path $StageDir $file))) { Fail "missing $file next to the bench binary" }
    }

    $binDir = Join-Path $BenchHome 'bin'
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    Copy-Item -Force $StageBin (Join-Path $binDir 'bench.exe')
    Copy-Item -Force (Join-Path $StageDir 'NOTICE'), (Join-Path $StageDir 'LICENSE') $BenchHome
    $installedSkill = Join-Path $BenchHome "skills\$Skill"
    Copy-Dir $StageSkill $installedSkill

    $config = Join-Path $BenchHome 'config.toml'
    if (-not (Test-Path $config)) {
        Set-Content -Path $config -Encoding ascii -Value @(
            '# bench settings. See the homebrew-bench skill for what each one does.',
            'output = "home"',
            'keep_encounters = true',
            'delete_logs = false',
            'round_cap = 20'
        )
    }
    $index = Join-Path $BenchHome 'index'
    if (-not (Test-Path $index)) {
        Set-Content -Path $index -Encoding ascii -Value @('{', '  "encounters": []', '}')
    }

    Write-Host "Installed bench to $(Join-Path $binDir 'bench.exe')"

    if (-not $NoSkills) {
        if ($Agents.Count -eq 0) {
            $Agents.Add('agents')
            foreach ($agent in 'claude', 'cline', 'kiro') {
                if ((Get-Command $agent -ErrorAction SilentlyContinue) -or (Test-Path (Join-Path $UserHome ".$agent"))) {
                    $Agents.Add($agent)
                }
            }
        }
        foreach ($agent in ($Agents | Select-Object -Unique)) {
            $dest = Join-Path $UserHome ".$agent\skills\$Skill"
            Copy-Dir $installedSkill $dest
            Write-Host "Installed the $Skill skill to $dest"
        }
    }

    $pathDirs = ($env:PATH -split ';') | ForEach-Object { $_.TrimEnd('\') }
    if ($pathDirs -notcontains $binDir.TrimEnd('\')) {
        Write-Host "Add bench to your PATH: [Environment]::SetEnvironmentVariable('PATH', `"$binDir;`" + [Environment]::GetEnvironmentVariable('PATH', 'User'), 'User')"
    }
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
