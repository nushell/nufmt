{
  description = "The Nushell Formatter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
      pkgsFor = forEachSystem (
        system:
        import nixpkgs {
          inherit system;
          config.allowDeprecatedx86_64Darwin = true;
        }
      );
    in
    {
      devShells = forEachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.default ];
            packages = with pkgs; [
              nushell

              # Not included in the package dependencies, but used for development
              rust-analyzer
              rustfmt
              clippy
            ];
          };
        }
      );

      packages = forEachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
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
          nufmt = pkgs.rustPlatform.buildRustPackage {
            name = "nufmt";
            src = ./.;
            patches = [
              (pkgs.replaceVars "${integrationTestPatch}" {
                target_triple = pkgs.stdenv.hostPlatform.rust.rustcTarget;
              })
            ];
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "A formatter for Nushell scripts, built entirely on Nushell's own parsing infrastructure";
              homepage = "https://github.com/nushell/nufmt";
              license = pkgs.lib.licenses.mit;
              mainProgram = "nufmt";
            };
          };
        in
        {
          default = nufmt;
          inherit nufmt;
        }
      );

      formatter = forEachSystem (system: pkgsFor.${system}.nixfmt-tree);
    };
}
