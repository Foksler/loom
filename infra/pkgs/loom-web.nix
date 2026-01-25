# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ lib
, stdenv
, nodejs_22
, pnpm_9
, pnpmConfigHook
, fetchPnpmDeps
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "loom-web";
  version = "0.1.0";

  src = ../../web;

  nativeBuildInputs = [
    nodejs_22
    pnpm_9
    pnpmConfigHook
  ];

  pnpmDeps = fetchPnpmDeps {
    inherit (finalAttrs) pname version src;
    pnpm = pnpm_9;
    fetcherVersion = 2;
    hash = "sha256-xHzPvJ4IPLIk3KXlT8UumzA9m2E/OJLIXHxm2fvjTb0=";
  };

  buildPhase = ''
    runHook preBuild
    # Build workspace dependencies first
    pnpm --filter @loom/http run build
    pnpm --filter @loom/crash run build
    pnpm --filter @loom/analytics run build
    # Then build loom-web
    pnpm --filter @loom/web run lingui:compile
    pnpm --filter @loom/web run build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/loom-web
    cp -r loom-web/build/* $out/share/loom-web/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Loom web application - Browser-based interface for Loom AI coding assistant";
    longDescription = ''
      Loom Web is a Svelte 5 web application that provides a browser-based
      interface for the Loom AI coding agent system. It enables users to view,
      manage, and interact with conversation threads, visualize agent state
      transitions, and monitor tool executions in real-time.
    '';
    homepage = "https://github.com/ghuntley/loom";
    license = licenses.unfree;
    maintainers = [ ];
  };
})
