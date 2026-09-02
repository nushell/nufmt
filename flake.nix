{
  description = "The Nushell formatter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
          nufmt = rustPlatform.buildRustPackage {
            pname = "nufmt";
            version = "0.1.4";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "nu-derive-value-0.115.2" = "sha256-PwX5CHTBf0dkG5ggFAkS/7cVc3eDeyysBBaeRriHG7Y=";
                "proc-macro-error3-3.1.1" = "sha256-/vZmJXRSlag84GL6/pwC3qSbnkENFtOLMzpZX8T2GTQ=";
              };
            };

            meta = {
              description = "A formatter for Nushell scripts";
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

      checks = forAllSystems (system: {
        nufmt = self.packages.${system}.nufmt;
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "clippy"
              "rust-analyzer"
              "rustfmt"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.nushell
            ];
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        pkgs.writeShellApplication {
          name = "format-nix";
          text = ''
            ${pkgs.nixfmt}/bin/nixfmt flake.nix
          '';
        }
      );
    };
}
