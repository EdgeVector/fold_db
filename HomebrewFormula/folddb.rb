class Folddb < Formula
  desc "Personal data cloud - sync files, query with AI, build apps on your data"
  homepage "https://github.com/shiba4life/fold_db"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb-macos-aarch64-#{version}.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb-macos-x86_64-#{version}.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb-linux-x86_64-#{version}.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  resource "folddb_server" do
    on_macos do
      on_arm do
        url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb_server-macos-aarch64-#{version}.tar.gz"
        sha256 "PLACEHOLDER"
      end
      on_intel do
        url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb_server-macos-x86_64-#{version}.tar.gz"
        sha256 "PLACEHOLDER"
      end
    end
    on_linux do
      on_intel do
        url "https://github.com/shiba4life/fold_db/releases/download/v#{version}/folddb_server-linux-x86_64-#{version}.tar.gz"
        sha256 "PLACEHOLDER"
      end
    end
  end

  def install
    bin.install Dir["folddb-*"].first => "folddb"

    resource("folddb_server").stage do
      bin.install Dir["folddb_server-*"].first => "folddb_server"
    end
  end

  test do
    assert_match "folddb", shell_output("#{bin}/folddb --help")
  end
end
