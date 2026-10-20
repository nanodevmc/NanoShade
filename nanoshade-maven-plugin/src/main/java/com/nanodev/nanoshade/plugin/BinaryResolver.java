package com.nanodev.nanoshade.plugin;

import org.apache.maven.plugin.logging.Log;

import java.io.File;
import java.io.InputStream;
import java.io.IOException;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public class BinaryResolver {
    private static final String GITHUB_RELEASE_BASE =
            "https://github.com/NanoDev1488/NanoShade/releases/latest/download/";
    public static final String FLATPAK_APP_ID = "com.nanodev.NanoShade";

    public static List<String> resolveCommandLine(String customPath, boolean forceFlatpak, Log log) throws IOException {
        String os = System.getProperty("os.name").toLowerCase();
        boolean isLinux = os.contains("linux");

        // 1. Проверяем режим Flatpak
        if (forceFlatpak || (isLinux && isFlatpakInstalled(log))) {
            log.info("Using Flatpak sandbox runner for NanoShade (" + FLATPAK_APP_ID + ")...");
            List<String> cmd = new ArrayList<>();
            cmd.add("flatpak");
            cmd.add("run");
            cmd.add(FLATPAK_APP_ID);
            return cmd;
        }

        // 2. Нативный бинарник под текущую ОС и архитектуру
        File bin = resolveNativeBinary(customPath, log);
        return Collections.singletonList(bin.getAbsolutePath());
    }

    private static boolean isFlatpakInstalled(Log log) {
        try {
            Process process = new ProcessBuilder("flatpak", "info", FLATPAK_APP_ID).start();
            int exit = process.waitFor();
            return exit == 0;
        } catch (Exception e) {
            return false;
        }
    }

    public static File resolveNativeBinary(String customPath, Log log) throws IOException {
        if (customPath != null && !customPath.trim().isEmpty()) {
            File custom = new File(customPath);
            if (custom.exists()) {
                return custom;
            }
            log.warn("Custom native binary path does not exist: " + customPath);
        }

        String os = System.getProperty("os.name").toLowerCase();
        String arch = System.getProperty("os.arch").toLowerCase();
        boolean isArm = arch.contains("aarch64") || arch.contains("arm64");

        boolean isWindows = os.contains("win");
        boolean isLinux = os.contains("linux");
        boolean isMac = os.contains("mac");

        String binName;
        String resourcePath;
        if (isWindows) {
            if (isArm) {
                binName = "nanoshade-windows-arm64.exe";
                resourcePath = "/binaries/windows-arm64/nanoshade.exe";
            } else {
                binName = "nanoshade.exe";
                resourcePath = "/binaries/windows-x86_64/nanoshade.exe";
            }
        } else if (isLinux) {
            if (isArm) {
                binName = "nanoshade-linux-arm64";
                resourcePath = "/binaries/linux-arm64/nanoshade";
            } else {
                binName = "nanoshade";
                resourcePath = "/binaries/linux-x86_64/nanoshade";
            }
        } else if (isMac) {
            if (isArm) {
                binName = "nanoshade-macos-arm64";
                resourcePath = "/binaries/macos-arm64/nanoshade";
            } else {
                binName = "nanoshade-macos-x86_64";
                resourcePath = "/binaries/macos-x86_64/nanoshade";
            }
        } else {
            throw new UnsupportedOperationException("Unsupported OS: " + os + " / Arch: " + arch);
        }

        // 1. Проверяем локальный кэш пользователя ~/.nanoshade/bin/
        String userHome = System.getProperty("user.home");
        Path cacheDir = Paths.get(userHome, ".nanoshade", "bin");
        Files.createDirectories(cacheDir);
        File cachedBin = cacheDir.resolve(binName).toFile();
        if (cachedBin.exists() && cachedBin.length() > 0) {
            cachedBin.setExecutable(true);
            return cachedBin;
        }

        // 2. Проверяем встроенный ресурс внутри плагина
        try (InputStream in = BinaryResolver.class.getResourceAsStream(resourcePath)) {
            if (in != null) {
                log.info("Extracting embedded native binary for " + os + " (" + arch + ")...");
                Files.copy(in, cachedBin.toPath(), StandardCopyOption.REPLACE_EXISTING);
                cachedBin.setExecutable(true);
                return cachedBin;
            }
        }

        // 3. Скачиваем релизный бинарник под текущую архитектуру с GitHub Releases
        String downloadUrl = GITHUB_RELEASE_BASE + binName;
        log.info("Downloading native NanoShade binary for " + os + " (" + arch + ") from " + downloadUrl + " ...");
        try (InputStream in = new URL(downloadUrl).openStream()) {
            Files.copy(in, cachedBin.toPath(), StandardCopyOption.REPLACE_EXISTING);
            cachedBin.setExecutable(true);
            log.info("Successfully downloaded and cached at " + cachedBin.getAbsolutePath());
            return cachedBin;
        } catch (Exception e) {
            throw new IOException("Failed to resolve or download native NanoShade binary for " + os + " (" + arch + "): " + e.getMessage(), e);
        }
    }
}
