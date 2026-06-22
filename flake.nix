{
  description = "Pure Rust RPM repository metadata generator — dnf/yum-compatible, zero FFI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems =
        f: nixpkgs.lib.genAttrs systems (system: f system nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        system: pkgs: rec {
          default = createrepo-rs;
          createrepo-rs = pkgs.rustPlatform.buildRustPackage {
            pname = "createrepo-rs";
            version = "0.1.9";

            src = ./.;

            cargoLock.lockFile = ./Cargo.lock;

            __structuredAttrs = true;

            meta = {
              description = "Pure Rust RPM repository metadata generator — dnf/yum-compatible, zero FFI";
              homepage = "https://github.com/artifactx-rs/createrepo_rs";
              changelog = "https://github.com/artifactx-rs/createrepo_rs/releases";
              license = pkgs.lib.licenses.gpl2Plus;
              mainProgram = "createrepo_rs";
              maintainers = [ ];
              platforms = pkgs.lib.platforms.linux ++ pkgs.lib.platforms.darwin;
            };
          };
        }
      );

      devShells = forAllSystems (
        system: pkgs: {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.clippy
            ];
          };
        }
      );

      formatter = forAllSystems (system: pkgs: pkgs.nixfmt-rfc-style);
    };
}
