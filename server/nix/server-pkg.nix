{
  lib,
  rustPlatform,
  nix,
  pkg-config,
  openssl,
  makeWrapper,
  src,
}:

rustPlatform.buildRustPackage {
  pname = "nix-repl-server";
  version = "0.1.0";

  inherit src;

  cargoLock.lockFile = "${src}/Cargo.lock";

  # Project has 2 Cargo.toml files
  postPatch = ''
    cp Cargo.toml.inc Cargo.toml
  '';

  # Runtime dependencies (nix for evaluation)
  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];
  buildInputs = [ openssl ];

  doCheck = false;

  # Ensure 'nix' is available in the path if your binary calls Command::new("nix")
  postInstall = ''
    wrapProgram $out/bin/nix-repl-server --prefix PATH : ${lib.makeBinPath [ nix ]}
  '';

  meta = with lib; {
    description = "Secure Nix REPL server for mdbook-nix-repl";
    platforms = platforms.linux;
  };
}
