cask "evita" do
  version "1.1.0-nightly.20261128.30"
  sha256 "27424b523187d8a5f80f04b565e93cdedac84e514d2fe7805ade927ec6755242"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
