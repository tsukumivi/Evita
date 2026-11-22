cask "evita" do
  version "1.1.0-nightly.20261122.24"
  sha256 "fdafeee857fe5aae36df3ec5479b3addfc2a8cd45acdf6eed6b1a44bd383d28e"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
