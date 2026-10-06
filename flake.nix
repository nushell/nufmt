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
      # The nushell 0.116 crates need Rust 1.96.1 or newer, and nixos-26.05
      # defaults to Rust 1.95.
      rustPackagesFor = system: pkgsFor.${system}.rustPackages_1_97;
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
              (rustPackagesFor system).rustfmt
              (rustPackagesFor system).clippy
            ];
          };
        }
      );

      packages = forEachSystem (
        system:
        let
          pkgs = pkgsFor.${system};
          nufmt = (rustPackagesFor system).rustPlatform.buildRustPackage {
            name = "nufmt";
            src = ./.;
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
