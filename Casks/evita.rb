cask "evita" do
  version "1.1.0-nightly.20261116.18"
  sha256 "5cc7e305b032e0fede371926f49688962740236be571480a123e2044ba805b97"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
