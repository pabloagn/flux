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
    cd core
    echo "Core shell (Rust) ready."
  '';
}
