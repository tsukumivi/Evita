# Viviette companion source

This is the Windows media Rich Presence companion bundled with Evita. It runs as a separate control-panel and tray process; it is not a Discord chat client embedded in Evita. When Evita is running and activity sharing is enabled, Viviette publishes presence through Evita's local Rich Presence IPC.

This source is adapted from the public Yozora project at https://github.com/mariyaholic/yozora, upstream commit a74b4c8897c349f1e4f39ac2461ec80c46cf175c (MIT License). The original copyright and license are preserved in UPSTREAM-LICENSE-MIT as required by the license. Product strings and executable branding in this copy are renamed to Viviette. Upstream Discord application metadata remains controlled by its publisher, so Discord may still display the original registered application name until an Evita-owned Discord application ID is configured.

Build on Windows with Go 1.27.1 or newer from this directory:

```powershell
$env:GOOS = "windows"
$env:GOARCH = "amd64"
New-Item -ItemType Directory -Force ..\..\target\viviette | Out-Null
go build -mod=readonly -trimpath -ldflags="-s -w -H=windowsgui" -o ..\..\target\viviette\Viviette.exe .\cmd\viviette
```

Do not commit generated .exe or .syso files.
