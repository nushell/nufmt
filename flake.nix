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
              index c19811d..33c3829 100644
              --- a/tests/main.rs
              +++ b/tests/main.rs
              @@ -15,7 +15,7 @@ fn failure_with_invalid_config() {
                   let config_file = dir.path().join("nufmt.nuon");
                   fs::write(&config_file, r#"{unknown: 1}"#).unwrap();

              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--config")
                       .arg(config_file.to_str().unwrap())
                       .arg(dir.path().to_str().unwrap())
              @@ -29,7 +29,7 @@ fn failure_with_invalid_config() {

               #[test]
               fn failure_with_invalid_config_file() {
              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--config")
                       .arg("path/that/does/not/exist/nufmt.nuon")
                       .output()
              @@ -42,7 +42,7 @@ fn failure_with_invalid_config_file() {

               #[test]
               fn failure_with_invalid_file_to_format() {
              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("path/that/does/not/exist/a.nu")
                       .output()
                       .unwrap();
              @@ -56,7 +56,7 @@ fn failure_with_invalid_file_to_format() {
               fn warning_when_no_files_are_detected() {
                   let dir = tempdir().unwrap();

              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--dry-run")
                       .arg(dir.path().to_str().unwrap())
                       .output()
              @@ -75,7 +75,7 @@ fn warning_is_displayed_when_no_files_are_detected_with_excluded_files() {
                   fs::write(&config_file, r#"{exclude: ["a*"]}"#).unwrap();
                   fs::write(&file_a, INVALID).unwrap();

              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--config")
                       .arg(config_file.to_str().unwrap())
                       .arg("--dry-run")
              @@ -98,7 +98,7 @@ fn files_are_reformatted() {
                   fs::write(&file_a, INVALID).unwrap();
                   fs::write(&file_b, INVALID).unwrap();

              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--config")
                       .arg(config_file.to_str().unwrap())
                       .arg(dir.path().to_str().unwrap())
              @@ -122,7 +122,7 @@ fn files_are_checked() {
                   fs::write(&file_a, INVALID).unwrap();
                   fs::write(&file_b, INVALID).unwrap();

              -    let output = Command::new("target/debug/nufmt")
              +    let output = Command::new("target/@target_triple@/release/nufmt")
                       .arg("--config")
                       .arg(config_file.to_str().unwrap())
                       .arg("--dry-run")
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
