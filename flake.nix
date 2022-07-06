{
  description = "µbench dev flake";

  inputs = {
    nixpkgs.url      = github:NixOS/nixpkgs/nixos-21.11;
    rust-overlay.url = github:oxalica/rust-overlay;
    flake-utils.url  = github:numtide/flake-utils;
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rust-toolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml);
        llvm-tools-preview = builtins.head (builtins.filter (p: p.pname == "llvm-tools-preview") rust-toolchain.paths);

        # `gdb` is broken on ARM macOS so we'll fallback to using x86_64 GDB
        # there (assuming Rosetta is installed: https://github.com/NixOS/nix/pull/4310).
        #
        # See: https://github.com/NixOS/nixpkgs/issues/147953
        gdbPkgs' = let
          pkgs' = if pkgs.stdenv.isDarwin && pkgs.stdenv.isAarch64 then
            (import nixpkgs { system = "x86_64-darwin"; inherit overlays; })
          else
            pkgs;
        in
          [ pkgs'.gdb ]
        ;
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            rust-toolchain
            openocd
            picocom
          ] ++ gdbPkgs;
          shellHook = ''
          '';
        };
      }
    );
}
