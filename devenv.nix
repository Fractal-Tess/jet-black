{ pkgs, ... }:

{
  #=============================================================================
  # LANGUAGES
  #=============================================================================

  # JavaScript/TypeScript with Bun runtime
  languages.javascript = {
    enable = true;
    bun.enable = true;
  };

  #=============================================================================
  # SYSTEM PACKAGES
  #=============================================================================

  packages = with pkgs; [ ];

  #=============================================================================
  # ENVIRONMENT VARIABLES
  #=============================================================================

  env = {
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD = "1";
    PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
  };

  enterShell = ''
    # Welcome banner
    ${pkgs.figlet}/bin/figlet -f slant "monorepo" 2>/dev/null | ${pkgs.lolcat}/bin/lolcat
    echo "bun" $(${pkgs.bun}/bin/bun --version 2>/dev/null) | ${pkgs.lolcat}/bin/lolcat
    echo ""

    # Resolve Playwright Chromium executable path
    export PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=$(
      find ${pkgs.playwright-driver.browsers}/chromium-*/chrome-linux*/ \
        -name chrome -type f | head -1
    )
  '';

  #=============================================================================
  # SCRIPTS (available in devenv shell)
  #=============================================================================

  scripts = {
    dev.exec = "bun run dev";
    build.exec = "infisical run -- turbo build";
    test.exec = "infisical run -- turbo test";
    lint.exec = "bun run lint";
    format.exec = "bunx ultracite fix convex";
    typecheck.exec = "infisical run -- turbo typecheck";

    seed.exec = "bun run --cwd convex seed";
    convex_dashboard.exec = "bun run --cwd convex dashboard";
  };

  #=============================================================================
  # PROCESSES (start with `devenv up`)
  #=============================================================================

  processes = {
    web.exec = "bun run --cwd apps/web dev";
    convex.exec = "bun run --cwd convex dev";
  };
}
