{
  description = "The Nushell Formatter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import inputs.rust-overlay) ];
        pkgs = import inputs.nixpkgs {
          inherit system overlays;
        };
        integrationTestPatch = pkgs.writeTextFile {
          name = "integration-tests-nix-fix.patch";
          text =
            # patch
            ''
              diff --git a/tests/main.rs b/tests/main.rs
              index c3a2b64..173aea0 100644
              --- a/tests/main.rs
              +++ b/tests/main.rs
              @@ -8,7 +8,7 @@ let one = 1
               const VALID: &str = "# beginning of script comment
               let one = 1
               ";
              -const TEST_BINARY: &'static str = "target/debug/nufmt";
              +const TEST_BINARY: &'static str = "target/@target_triple@/release/nufmt";

               #[test]
               fn failure_with_invalid_config() {
            '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [
            nushell

            # Not included in the package dependencies, but used for development
            cargo-watch
            clippy
            rustfmt
            rust-analyzer
          ];
        };

        packages.default = self.packages.${system}.nufmt;
        packages.nufmt = pkgs.rustPlatform.buildRustPackage {
          name = "nufmt";
          src = ./.;
          patches = [
            (pkgs.replaceVars "${integrationTestPatch}" {
              target_triple = pkgs.stdenv.hostPlatform.rust.rustcTarget;
            })
          ];
          cargoLock.lockFile = ./Cargo.lock;
        };

        formatter = inputs.treefmt-nix.lib.mkWrapper pkgs {
          programs.nixfmt.enable = true;
        };
      }
    );
}
