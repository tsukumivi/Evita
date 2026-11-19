cask "evita" do
  version "1.1.0-nightly.20261119.21"
  sha256 "0d877d5a35309bcf500cf3e535a86ea9ef83aab5376ccef8f5063a6e035169e8"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
