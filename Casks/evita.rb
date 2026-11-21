cask "evita" do
  version "1.1.0-nightly.20261121.23"
  sha256 "e6159ca2cc1c5be045a3b95ecd6dbf8ba0c54bc3b868d185ef03e09f3b004a73"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
