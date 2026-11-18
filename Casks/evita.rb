cask "evita" do
  version "1.1.0-nightly.20261118.20"
  sha256 "fde5c32fbcf3879677771c66df8ab42b9921d63f1e4db6e35d9cc0f0b01ece8b"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
