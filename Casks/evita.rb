cask "evita" do
  version "1.1.0-nightly.20261109.8"
  sha256 "85fa09df504714c326f90dddf54cf5b1d5a464ba88f5c767096e9a3ceb1761fb"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
