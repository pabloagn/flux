{
  pkgs,
  rustToolchain,
  pythonBase,
  libsPath,
  systemPackages,
}:
{
  # --- Default Shell ---
  default = import ./shell-default.nix {
    inherit
      pkgs
      rustToolchain
      pythonBase
      libsPath
      systemPackages
      ;
  };

  # --- Application Shells ---
  app-tui = import ./shell-app-tui.nix {
    inherit
      pkgs
      rustToolchain
      libsPath
      systemPackages
      ;
  };

  app-web = import ./shell-app-web.nix {
    inherit
      pkgs
      libsPath
      systemPackages
      ;
  };

  # --- Service Shells ---
  srv-sim = import ./shell-srv-sim.nix {
    inherit
      pkgs
      pythonBase
      libsPath
      systemPackages
      ;
  };

  srv-kpi = import ./shell-srv-kpi.nix {
    inherit
      pkgs
      pythonBase
      libsPath
      systemPackages
      ;
  };

  srv-data-pipeline = import ./shell-srv-data-pipeline.nix {
    inherit
      pkgs
      pythonBase
      systemPackages
      ;
  };
}
