param(
    [Parameter(Mandatory)]
    [string] $InstallRoot,
    [string] $TargetDirectory = (Join-Path $PSScriptRoot '../target')
)

$ErrorActionPreference = 'Stop'
$buildRoot = [System.IO.Path]::GetFullPath($TargetDirectory)
$destination = [System.IO.Path]::GetFullPath($InstallRoot)
cargo install --path $PSScriptRoot --root $destination --target-dir $buildRoot --target x86_64-pc-windows-msvc --locked
if ($LASTEXITCODE -ne 0) {
    throw "Gallery installation failed: $LASTEXITCODE"
}

$releaseDirectory = Join-Path $buildRoot 'x86_64-pc-windows-msvc/release'
$builtExecutable = Join-Path $releaseDirectory 'widget_gallery.exe'
$installedExecutable = Join-Path $destination 'bin/widget_gallery.exe'
if ((Get-FileHash -LiteralPath $builtExecutable).Hash -ne (Get-FileHash -LiteralPath $installedExecutable).Hash) {
    throw 'Installed Gallery does not match the build artifacts; refusing to copy mismatched symbols'
}
Copy-Item -LiteralPath (Join-Path $releaseDirectory 'widget_gallery.pdb') -Destination (Join-Path $destination 'bin/widget_gallery.pdb')
Write-Output $installedExecutable
