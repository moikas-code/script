# Script Language Installer for Windows
# Requires PowerShell 5.0 or later

[CmdletBinding()]
param(
    [string]$Version = "",
    [string]$InstallDir = "$env:LOCALAPPDATA\script\bin",
    [switch]$NoPath,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# Configuration
$RepoOwner = "moikapy"
$RepoName = "script"
$GitHubApi = "https://api.github.com/repos/$RepoOwner/$RepoName"

# Colors and formatting
function Write-Info {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Blue -NoNewline
    Write-Host $Message -ForegroundColor White
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host $Message -ForegroundColor Red
}

function Write-Warning {
    param([string]$Message)
    Write-Host "! " -ForegroundColor Yellow -NoNewline
    Write-Host $Message -ForegroundColor Yellow
}

# Check requirements
function Test-Requirements {
    Write-Info "Checking requirements..."
    
    # Check PowerShell version
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Write-Error "PowerShell 5.0 or later is required"
        exit 1
    }
    
    # Check if running as admin (optional, but helpful for PATH updates)
    $isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")
    if (-not $isAdmin) {
        Write-Warning "Not running as administrator. PATH update will be for current user only."
    }
    
    Write-Success "All requirements met"
}

# Get the latest release version
function Get-LatestVersion {
    Write-Info "Fetching latest version..."
    
    try {
        $release = Invoke-RestMethod -Uri "$GitHubApi/releases/latest" -Headers @{
            "User-Agent" = "Script-Installer"
        }
        
        $version = $release.tag_name
        Write-Success "Latest version: $version"
        return $version
    }
    catch {
        Write-Error "Failed to fetch latest version: $_"
        exit 1
    }
}

# Download file with progress
function Download-WithProgress {
    param(
        [string]$Url,
        [string]$OutputPath
    )
    
    try {
        $ProgressPreference = 'SilentlyContinue'  # Faster downloads
        Invoke-WebRequest -Uri $Url -OutFile $OutputPath -UseBasicParsing
        $ProgressPreference = 'Continue'
    }
    catch {
        Write-Error "Failed to download from $Url : $_"
        exit 1
    }
}

# Install Script Language
function Install-ScriptLang {
    param([string]$TargetVersion)
    
    if (-not $TargetVersion) {
        $TargetVersion = Get-LatestVersion
    }
    
    Write-Info "Installing Script Language $TargetVersion"
    
    # Create temp directory
    $tempDir = Join-Path $env:TEMP "script-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    
    try {
        # Determine architecture
        $arch = if ([Environment]::Is64BitOperatingSystem) { "amd64" } else { "x86" }
        $archiveName = "script-windows-$arch.zip"
        $downloadUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$TargetVersion/$archiveName"
        
        Write-Info "Downloading from: $downloadUrl"
        $archivePath = Join-Path $tempDir $archiveName
        Download-WithProgress -Url $downloadUrl -OutputPath $archivePath
        
        # Download and verify checksum if available
        $checksumUrl = "$downloadUrl.sha256"
        $checksumPath = "$archivePath.sha256"
        
        try {
            Download-WithProgress -Url $checksumUrl -OutputPath $checksumPath
            Write-Info "Verifying checksum..."
            
            $expectedChecksum = (Get-Content $checksumPath -Raw).Trim().Split(' ')[0]
            $actualChecksum = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash
            
            if ($expectedChecksum -eq $actualChecksum) {
                Write-Success "Checksum verified"
            } else {
                Write-Error "Checksum mismatch!"
                exit 1
            }
        }
        catch {
            Write-Warning "Checksum file not found, skipping verification"
        }
        
        # Extract archive
        Write-Info "Extracting archive..."
        Expand-Archive -Path $archivePath -DestinationPath $tempDir -Force
        
        # Create install directory
        if (-not (Test-Path $InstallDir)) {
            Write-Info "Creating install directory: $InstallDir"
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        # Install binaries
        Write-Info "Installing binaries to $InstallDir..."
        
        $binaries = @("script.exe", "script-lsp.exe", "manuscript.exe")
        foreach ($binary in $binaries) {
            $sourcePath = Join-Path $tempDir $binary
            if (Test-Path $sourcePath) {
                $destPath = Join-Path $InstallDir $binary
                
                # Backup existing binary if updating
                if (Test-Path $destPath -and -not $Force) {
                    $backupPath = "$destPath.backup"
                    Move-Item -Path $destPath -Destination $backupPath -Force
                }
                
                Copy-Item -Path $sourcePath -Destination $destPath -Force
                Write-Success "Installed $binary"
            } else {
                Write-Warning "$binary not found in archive"
            }
        }
    }
    finally {
        # Cleanup temp directory
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Update PATH
function Update-Path {
    if ($NoPath) {
        Write-Info "Skipping PATH update (--NoPath specified)"
        return
    }
    
    # Check if already in PATH
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -like "*$InstallDir*") {
        Write-Info "Install directory already in PATH"
        return
    }
    
    Write-Info "Adding $InstallDir to PATH..."
    
    try {
        $newPath = "$currentPath;$InstallDir"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        
        # Update current session
        $env:Path = "$env:Path;$InstallDir"
        
        Write-Success "PATH updated successfully"
        Write-Warning "Restart your terminal or run 'refreshenv' to use the new PATH"
    }
    catch {
        Write-Warning "Failed to update PATH automatically"
        Write-Warning "Please add the following directory to your PATH manually:"
        Write-Host "    $InstallDir" -ForegroundColor Cyan
    }
}

# Create Start Menu shortcut
function New-StartMenuShortcut {
    Write-Info "Creating Start Menu shortcuts..."
    
    $startMenuPath = [Environment]::GetFolderPath("StartMenu")
    $scriptLangFolder = Join-Path $startMenuPath "Programs\Script Language"
    
    if (-not (Test-Path $scriptLangFolder)) {
        New-Item -ItemType Directory -Path $scriptLangFolder -Force | Out-Null
    }
    
    try {
        $WshShell = New-Object -ComObject WScript.Shell
        
        # Script Language REPL shortcut
        $shortcut = $WshShell.CreateShortcut("$scriptLangFolder\Script Language REPL.lnk")
        $shortcut.TargetPath = Join-Path $InstallDir "script.exe"
        $shortcut.WorkingDirectory = "%USERPROFILE%"
        $shortcut.Description = "Script Language Interactive REPL"
        $shortcut.Save()
        
        # Manuscript shortcut
        $shortcut = $WshShell.CreateShortcut("$scriptLangFolder\Manuscript Package Manager.lnk")
        $shortcut.TargetPath = Join-Path $InstallDir "manuscript.exe"
        $shortcut.WorkingDirectory = "%USERPROFILE%"
        $shortcut.Description = "Script Language Package Manager"
        $shortcut.Save()
        
        Write-Success "Start Menu shortcuts created"
    }
    catch {
        Write-Warning "Failed to create Start Menu shortcuts: $_"
    }
}

# Main installation process
function Main {
    Write-Host ""
    Write-Host "Script Language Installer for Windows" -ForegroundColor Cyan
    Write-Host ""
    
    Test-Requirements
    Install-ScriptLang -TargetVersion $Version
    Update-Path
    New-StartMenuShortcut
    
    Write-Host ""
    Write-Success "Script Language installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Run 'script --version' to verify the installation" -ForegroundColor White
    Write-Host "Run 'script update' to check for updates" -ForegroundColor White
    Write-Host ""
    
    # Test if script is accessible
    try {
        $testPath = Join-Path $InstallDir "script.exe"
        if (Test-Path $testPath) {
            & $testPath --version
        }
    }
    catch {
        # Ignore errors in version check
    }
}

# Run main installation
Main