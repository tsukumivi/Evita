cask "evita" do
  version "1.0.1-nightly.20261108.4"
  sha256 "5f17f31a39d978af98df18288364e7f5673eb9f088d547e15aeb10a50120acc3"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
