param(
    [string]$MatlabRoot = $env:MATLABROOT
)

$ErrorActionPreference = 'Stop'
if (-not $MatlabRoot) {
    throw 'Set MATLABROOT or pass -MatlabRoot.'
}

$clang = (Get-Command clang.exe -ErrorAction Stop).Source
$root = Split-Path -Parent $PSScriptRoot
$shim = Join-Path $root 'native\mat_shim.c'
$ffi = Join-Path $root 'src\ffi.rs'
$coverage = Join-Path $root 'docs\API_COVERAGE.md'
$astFile = Join-Path ([System.IO.Path]::GetTempPath()) "matrust-api-$PID.ast"

try {
    & $clang '-DTARGET_API_VERSION=800' '-fsyntax-only' '-Xclang' '-ast-dump' `
        '-I' (Join-Path $MatlabRoot 'extern\include') $shim 2>$null |
        Set-Content -LiteralPath $astFile
    if ($LASTEXITCODE -ne 0) {
        throw "Clang header scan failed with exit code $LASTEXITCODE."
    }

    $ast = Get-Content -LiteralPath $astFile -Raw
    $declared = [regex]::Matches(
        $ast,
        "FunctionDecl[^\r\n]*\b((?:mx|mex|mat)[A-Za-z0-9_]+)\s+'"
    ) | ForEach-Object {
        $_.Groups[1].Value -replace '_(?:700|730|800)$', ''
    } | Where-Object {
        $_ -notlike 'matrust_*' -and $_ -ne 'mexFunction'
    } | Sort-Object -Unique

    $shimText = Get-Content -LiteralPath $shim -Raw
    $used = [regex]::Matches(
        $ast,
        "DeclRefExpr[^\r\n]*\bFunction\b[^\r\n]*'((?:mx|mex|mat)[A-Za-z0-9_]+)'"
    ) | ForEach-Object {
        $_.Groups[1].Value -replace '_(?:700|730|800)$', ''
    } | Sort-Object -Unique

    $documentedText = Get-Content -LiteralPath $coverage -Raw
    $missingShim = @($declared | Where-Object { $_ -notin $used })
    $missingDocs = @($declared | Where-Object {
        $documentedText -notmatch "(?<![A-Za-z0-9_])$([regex]::Escape($_))(?![A-Za-z0-9_])"
    })

    $cWrappers = [regex]::Matches(
        $shimText,
        '(?m)^[A-Za-z_][A-Za-z0-9_ *]*\b(matrust_[a-z0-9_]+)\s*\('
    ) | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
    $ffiText = Get-Content -LiteralPath $ffi -Raw
    $ffiWrappers = [regex]::Matches(
        $ffiText,
        '(?:pub\s+)?fn\s+(matrust_[a-z0-9_]+)\s*\('
    ) | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
    $linkedSymbols = [regex]::Matches(
        $ffiText,
        '#\[link_name\s*=\s*"(matrust_[a-z0-9_]+)"\]'
    ) | ForEach-Object { $_.Groups[1].Value }
    $ffiSymbols = @($ffiWrappers) + @($linkedSymbols) | Sort-Object -Unique
    $missingFfi = @($cWrappers | Where-Object { $_ -notin $ffiSymbols })

    if ($missingShim -or $missingDocs -or $missingFfi) {
        if ($missingShim) { Write-Error "Missing shim coverage: $($missingShim -join ', ')" }
        if ($missingDocs) { Write-Error "Missing documentation: $($missingDocs -join ', ')" }
        if ($missingFfi) { Write-Error "Missing Rust FFI declarations: $($missingFfi -join ', ')" }
        exit 1
    }

    $mat = @($declared | Where-Object { $_ -cmatch '^mat[A-Z]' }).Count
    $mex = @($declared | Where-Object { $_ -cmatch '^mex[A-Z]' }).Count
    $matrix = @($declared | Where-Object { $_ -cmatch '^mx[A-Z]' }).Count
    Write-Output "MATRUST_API_COVERAGE_PASS total=$($declared.Count) mat=$mat mex=$mex matrix=$matrix"
}
finally {
    Remove-Item -LiteralPath $astFile -ErrorAction SilentlyContinue
}
