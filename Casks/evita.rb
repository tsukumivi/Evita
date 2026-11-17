cask "evita" do
  version "1.1.0-nightly.20261117.19"
  sha256 "6ee58a801f25cec7f5212c3bf3c324a91f2795f25bfb4de1f4de61a8f3a435ee"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
