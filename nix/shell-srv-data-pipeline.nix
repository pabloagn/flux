{
  pkgs,
  pythonBase,
  systemPackages,
}:

pkgs.mkShell {
  name = "shell-srv-data-pipeline";

  buildInputs =
    [
      pythonBase
      pkgs.uv
    ]
    ++ systemPackages
    ++ [
    ];

  shellHook = ''
    export FLUX_ROOT=$(git rev-parse --show-toplevel)
    export PYTHONPATH="$FLUX_ROOT/services/data-pipeline/src:$PYTHONPATH"
    export UV_NO_PROGRESS=true
  '';
}
