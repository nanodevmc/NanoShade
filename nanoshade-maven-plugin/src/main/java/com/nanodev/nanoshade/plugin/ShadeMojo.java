package com.nanodev.nanoshade.plugin;

import org.apache.maven.plugin.AbstractMojo;
import org.apache.maven.plugin.MojoExecutionException;
import org.apache.maven.plugin.MojoFailureException;
import org.apache.maven.plugins.annotations.Component;
import org.apache.maven.plugins.annotations.LifecyclePhase;
import org.apache.maven.plugins.annotations.Mojo;
import org.apache.maven.plugins.annotations.Parameter;
import org.apache.maven.project.MavenProject;
import org.apache.maven.project.MavenProjectHelper;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

@Mojo(name = "shade", defaultPhase = LifecyclePhase.PACKAGE, requiresProject = true, threadSafe = true)
public class ShadeMojo extends AbstractMojo {

    public static final String PLUGIN_VERSION = "1.0.0";

    @Parameter(defaultValue = "${project}", readonly = true, required = true)
    private MavenProject project;

    @Component
    private MavenProjectHelper projectHelper;

    @Parameter(property = "relocations")
    private List<Relocation> relocations = new ArrayList<>();

    @Parameter(property = "treeShake", defaultValue = "false")
    private boolean treeShake;

    @Parameter(property = "entryPoints")
    private List<String> entryPoints = new ArrayList<>();

    @Parameter(property = "keep")
    private List<String> keep = new ArrayList<>();

    @Parameter(property = "nativeExecutable")
    private String nativeExecutable;

    @Parameter(property = "useFlatpak", defaultValue = "false")
    private boolean useFlatpak;

    @Parameter(property = "shadedArtifactAttached", defaultValue = "false")
    private boolean shadedArtifactAttached;

    @Parameter(property = "shadedClassifierName", defaultValue = "shaded")
    private String shadedClassifierName;

    @Parameter(property = "outputFile")
    private File outputFile;

    @Parameter(property = "checkUpdates", defaultValue = "true")
    private boolean checkUpdates;

    @Override
    public void execute() throws MojoExecutionException, MojoFailureException {
        // Проверка обновлений NanoShade
        if (checkUpdates) {
            UpdateChecker.checkForUpdates(PLUGIN_VERSION, getLog());
        }

        File artifactFile = project.getArtifact().getFile();
        if (artifactFile == null || !artifactFile.exists()) {
            getLog().info("No primary artifact file found. Skipping NanoShade.");
            return;
        }

        File targetOutput = outputFile;
        if (targetOutput == null) {
            if (shadedArtifactAttached) {
                String name = artifactFile.getName();
                int dot = name.lastIndexOf('.');
                String base = (dot != -1) ? name.substring(0, dot) : name;
                String ext = (dot != -1) ? name.substring(dot) : ".jar";
                targetOutput = new File(artifactFile.getParentFile(), base + "-" + shadedClassifierName + ext);
            } else {
                targetOutput = new File(artifactFile.getParentFile(), "original-" + artifactFile.getName());
                // переименовываем оригинал
                if (!artifactFile.renameTo(targetOutput)) {
                    throw new MojoExecutionException("Failed to rename original artifact to " + targetOutput.getName());
                }
                File originalInput = targetOutput;
                targetOutput = artifactFile;
                artifactFile = originalInput;
            }
        }

        try {
            List<String> runnerCommand = BinaryResolver.resolveCommandLine(nativeExecutable, useFlatpak, getLog());
            getLog().info("Executing NanoShade engine: " + String.join(" ", runnerCommand));

            List<String> cmd = new ArrayList<>(runnerCommand);
            cmd.add("--input");
            cmd.add(artifactFile.getAbsolutePath());
            cmd.add("--output");
            cmd.add(targetOutput.getAbsolutePath());

            if (relocations != null) {
                for (Relocation r : relocations) {
                    cmd.add("--relocate");
                    cmd.add(r.toCliArgument());
                }
            }

            if (treeShake) {
                cmd.add("--tree-shake");
            }

            if (entryPoints != null) {
                for (String ep : entryPoints) {
                    cmd.add("--entry-point");
                    cmd.add(ep);
                }
            }

            if (keep != null) {
                for (String k : keep) {
                    cmd.add("--keep");
                    cmd.add(k);
                }
            }

            ProcessBuilder pb = new ProcessBuilder(cmd);
            pb.redirectErrorStream(true);
            Process process = pb.start();

            try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    getLog().info(line);
                }
            }

            int exitCode = process.waitFor();
            if (exitCode != 0) {
                throw new MojoExecutionException("NanoShade failed with exit code: " + exitCode);
            }

            if (shadedArtifactAttached) {
                projectHelper.attachArtifact(project, project.getArtifact().getType(), shadedClassifierName, targetOutput);
            }

        } catch (Exception e) {
            throw new MojoExecutionException("Error executing NanoShade: " + e.getMessage(), e);
        }
    }
}
