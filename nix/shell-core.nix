{
  pkgs,
  rustToolchain,
  libsPath,
  systemPackages,
  startScript,
  stopScript,
}:

pkgs.mkShell {
  buildInputs =
    [ rustToolchain ]
    ++ systemPackages
    ++ [
      startScript
      stopScript
    ];
  shellHook = ''
    export LD_LIBRARY_PATH='${libsPath}':$LD_LIBRARY_PATH
    export RUST_BACKTRACE=1
    export RUST_LOG=error
    cd core
    echo "Core shell (Rust) ready."
  '';
}
