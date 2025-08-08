{
  pkgs,
  rustToolchain,
  pythonBase,
  libsPath,
  systemPackages,
  startScript,
  stopScript,
  statusScript,
}:
{
  # Default shell
  default = import ./shell-default.nix {
    inherit
      pkgs
      rustToolchain
      pythonBase
      libsPath
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };

  # Application shells
  app-tui = import ./shell-app-tui.nix {
    inherit
      pkgs
      rustToolchain
      libsPath
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };

  app-web = import ./shell-app-web.nix {
    inherit
      pkgs
      libsPath
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };

  # Service shells
  srv-sim = import ./shell-srv-sim.nix {
    inherit
      pkgs
      pythonBase
      libsPath
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };

  srv-kpi = import ./shell-srv-kpi.nix {
    inherit
      pkgs
      pythonBase
      libsPath
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };

  srv-data-pipeline = import ./shell-srv-data-pipeline.nix {
    inherit
      pkgs
      pythonBase
      systemPackages
      startScript
      stopScript
      statusScript
      ;
  };
}
