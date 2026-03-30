class Folddb < Formula
  desc "Personal data sovereignty platform — CLI and server for FoldDB"
  homepage "https://github.com/EdgeVector/fold_db"
  version "0.3.0"
  license "MIT"

  FOLDDB_VERSION = "0.3.0"

  on_macos do
    on_arm do
      url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb-macos-aarch64-#{FOLDDB_VERSION}.tar.gz"
      sha256 "2b69979017b05d0602a762069e5a05e82fc1edc6a88eadc1348fe8bc76191f11"

      resource "folddb_server" do
        url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb_server-macos-aarch64-#{FOLDDB_VERSION}.tar.gz"
        sha256 "75b20a7dfa4cff2fd01b54eaf4c191c1051975f439453fb5b5f652dc8c47c281"
      end
    end
    on_intel do
      url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb-macos-x86_64-#{FOLDDB_VERSION}.tar.gz"
      sha256 "fa336444d5399d7b915c7ab25a05c2b20b5d7445209d66d36d489f58239ff397"

      resource "folddb_server" do
        url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb_server-macos-x86_64-#{FOLDDB_VERSION}.tar.gz"
        sha256 "ce9dee5b348e0efe9b1dcc76547944f693aac28e1a53b48169121d8d9cef4059"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb-linux-x86_64-#{FOLDDB_VERSION}.tar.gz"
      sha256 "4ecad09f4b31477c9263d6c7382a279969c9c3816c0929bcb5164bcf67f39d15"

      resource "folddb_server" do
        url "https://github.com/EdgeVector/fold_db/releases/download/v#{FOLDDB_VERSION}/folddb_server-linux-x86_64-#{FOLDDB_VERSION}.tar.gz"
        sha256 "bcdc21390cf2ddb3d45e9ba816bcdf9a99104611535175bb187b046bd5894dd8"
      end
    end
  end

  def install
    bin.install Dir["folddb*"].first => "folddb"

    resource("folddb_server").stage do
      bin.install Dir["folddb_server*"].first => "folddb_server"
    end
  end

  test do
    assert_match "folddb", shell_output("#{bin}/folddb --help", 2)
  end
end
