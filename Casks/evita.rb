cask "evita" do
  version "1.1.0-nightly.20261126.28"
  sha256 "1a41b61c5b3fe8a91337bd5d3ae074075a551fec032f6721ba77fb17380ff62e"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
