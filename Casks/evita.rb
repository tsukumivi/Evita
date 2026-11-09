cask "evita" do
  version "1.0.1-nightly.20261109.5"
  sha256 "b71ca5991fa0e2b7ac9ce4ca543c49c7f9715fd65b68b24b76ff876d5636b027"

  url "https://github.com/tsukumivi/Evita/releases/download/v#{version}/evita-v#{version}-macOS-ARM64.zip"
  name "Evita"
  desc "Experimental native Discord client"
  homepage "https://github.com/tsukumivi/Evita"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  app "Evita.app"
end
