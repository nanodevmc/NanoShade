package com.nanodev.nanoshade.plugin;

import org.apache.maven.plugin.logging.Log;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;

public class UpdateChecker {
    private static final String REPO_API = "https://api.github.com/repos/NanoDev1488/NanoShade/releases/latest";

    /**
     * Асинхронная проверка обновлений с жестким таймаутом, чтобы не тормозить сборку на слабых машинах
     */
    public static void checkForUpdates(String currentVersion, Log log) {
        if (currentVersion == null || currentVersion.endsWith("-SNAPSHOT")) {
            return;
        }

        Thread checkThread = new Thread(() -> {
            try {
                URL url = new URL(REPO_API);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("GET");
                conn.setRequestProperty("User-Agent", "NanoShade-UpdateChecker");
                conn.setRequestProperty("Accept", "application/vnd.github.v3+json");
                conn.setConnectTimeout(1500);
                conn.setReadTimeout(1500);

                int code = conn.getResponseCode();
                if (code == 200) {
                    try (BufferedReader reader = new BufferedReader(new InputStreamReader(conn.getInputStream()))) {
                        StringBuilder sb = new StringBuilder();
                        String line;
                        while ((line = reader.readLine()) != null) {
                            sb.append(line);
                        }
                        String json = sb.toString();
                        String latestTag = extractTag(json);
                        if (latestTag != null && isNewerVersion(currentVersion, latestTag)) {
                            log.warn("================================================================");
                            log.warn("[NanoShade] Вы используете устаревшую версию плагина: " + currentVersion);
                            log.warn("[NanoShade] Доступна новая версия: " + latestTag);
                            log.warn("[NanoShade] Рекомендуется обновить <version>" + latestTag + "</version> в pom.xml");
                            log.warn("[NanoShade] Подробнее: https://github.com/NanoDev1488/NanoShade/releases/latest");
                            log.warn("================================================================");
                        }
                    }
                }
            } catch (Exception ignored) {
                // Игнорируем сетевые сбои / оффлайн, чтобы не прерывать сборку проекта
            }
        });

        checkThread.setDaemon(true);
        checkThread.start();

        try {
            // Даем потоку максимум 600 мс, если успел - выведет сразу, если сеть медленная - сборка продолжается без задержки
            checkThread.join(600);
        } catch (InterruptedException ignored) {
            Thread.currentThread().interrupt();
        }
    }

    private static String extractTag(String json) {
        int idx = json.indexOf("\"tag_name\":");
        if (idx == -1) return null;
        int start = json.indexOf('"', idx + 11);
        if (start == -1) return null;
        int end = json.indexOf('"', start + 1);
        if (end == -1) return null;
        String raw = json.substring(start + 1, end).trim();
        return raw.startsWith("v") ? raw.substring(1) : raw;
    }

    public static boolean isNewerVersion(String current, String latest) {
        String[] cParts = current.split("[.-]");
        String[] lParts = latest.split("[.-]");
        int len = Math.max(cParts.length, lParts.length);
        for (int i = 0; i < len; i++) {
            int cVal = (i < cParts.length) ? parseSafe(cParts[i]) : 0;
            int lVal = (i < lParts.length) ? parseSafe(lParts[i]) : 0;
            if (lVal > cVal) return true;
            if (lVal < cVal) return false;
        }
        return false;
    }

    private static int parseSafe(String s) {
        try {
            return Integer.parseInt(s.replaceAll("\\D+", ""));
        } catch (NumberFormatException e) {
            return 0;
        }
    }
}
