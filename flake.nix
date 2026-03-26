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
              diff --git a/tests/ground_truth.rs b/tests/ground_truth.rs
              --- a/tests/ground_truth.rs
              +++ b/tests/ground_truth.rs
              @@ -14,11 +14,11 @@ pub fn get_test_binary() -> PathBuf {
                   // Try CARGO_TARGET_DIR first
                   if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
              -        let path = PathBuf::from(target_dir).join("debug").join(exe_name);
              +        let path = PathBuf::from(target_dir).join("@target_triple@/release").join(exe_name);
                       if path.exists() {
                           return path.canonicalize().unwrap_or(path);
                       }
                   }

                   // Try default target directory
              -    let default_path = PathBuf::from("target").join("debug").join(exe_name);
              +    let default_path = PathBuf::from("target").join("@target_triple@/release").join(exe_name);
                   if default_path.exists() {
            '';
        };
      in
      {
        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [
            nushell

            # Not included in the package dependencies, but used for development
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
