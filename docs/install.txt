# Installs Timeways on Windows: the Gnomish Relay installer with --timeways.
#   powershell -c "irm eserilev.github.io/timeways/install | iex"
# The file has no .ps1 so GitHub Pages serves it as text, which irm returns as a string.
try {
    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.ps1))) --timeways
} finally {
    # A window from Windows+R closes when the script ends, so the player reads the last line first.
    Read-Host "Press Enter to close"
}
