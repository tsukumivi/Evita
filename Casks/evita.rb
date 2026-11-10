cask "evita" do
  version "1.1.0-nightly.20261110.9"
  sha256 "8fd4baa26f73e042d0808df32505494308e4547bc10b79985eb2ba0a118ecb9b"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
