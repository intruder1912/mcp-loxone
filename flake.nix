{
  description = "MCP server for Loxone home automation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ];

        loxone-mcp = pkgs.rustPlatform.buildRustPackage {
          pname = "loxone-mcp";
          version = "0.7.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          # Skip tests that need network/live Miniserver
          checkFlags = [
            "--skip=live_miniserver"
          ];

          meta = {
            description = "MCP server for Loxone home automation";
            homepage = "https://github.com/avrabe/mcp-loxone";
            license = with pkgs.lib.licenses; [ mit asl20 ];
          };
        };
      in {
        packages = {
          default = loxone-mcp;
          loxone-mcp = loxone-mcp;
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
          RUST_LOG = "debug";
        };

        openclawPlugin = {
          name = "loxone";
          skills = [ ./skills/loxone ];
          packages = [ loxone-mcp ];
          needs = {
            stateDirs = [ ".config/loxone-mcp" ];
            requiredEnv = [ "LOXONE_HOST" "LOXONE_USER" "LOXONE_PASS" ];
          };
        };
      }
    );
}
