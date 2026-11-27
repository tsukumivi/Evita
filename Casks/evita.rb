cask "evita" do
  version "1.1.0-nightly.20261127.29"
  sha256 "192df4dcbbd6e9867618bc757d589dd4eab53c88312058d8fdb12598a5b08c54"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
