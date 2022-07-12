{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nmattia/naersk/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, naersk, flake-compat }:
    utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rust_stable = pkgs.rust-bin.stable.latest.default;
          naersk-lib = pkgs.callPackage naersk { rustc = rust_stable; cargo = rust_stable; };
          dependencies = [ pkgs.pkg-config pkgs.udev ];
          nativeBuildInputs = [ ];
        in
        {

          defaultPackage = naersk-lib.buildPackage
            {
              root = ./.;
              inherit nativeBuildInputs;
              buildInputs = dependencies;
            };

          defaultApp =
            let
              drv = self.defaultPackage."${system}";
              name = pkgs.lib.strings.removeSuffix ("-" + drv.version) drv.name;
            in
            utils.lib.mkApp {
              inherit drv;
              # TODO: https://github.com/nix-community/naersk/issues/224
              exePath = "/bin/${name}";
            };

          devShell = with pkgs;
            mkShell {
              inherit nativeBuildInputs;
              buildInputs = [ rust_stable ] ++ dependencies;
            };

        });
}
