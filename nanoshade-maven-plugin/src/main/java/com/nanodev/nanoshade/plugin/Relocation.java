package com.nanodev.nanoshade.plugin;

public class Relocation {
    private String pattern;
    private String shadedPattern;

    public Relocation() {
    }

    public Relocation(String pattern, String shadedPattern) {
        this.pattern = pattern;
        this.shadedPattern = shadedPattern;
    }

    public String getPattern() {
        return pattern;
    }

    public void setPattern(String pattern) {
        this.pattern = pattern;
    }

    public String getShadedPattern() {
        return shadedPattern;
    }

    public void setShadedPattern(String shadedPattern) {
        this.shadedPattern = shadedPattern;
    }

    public String toCliArgument() {
        return pattern + "=" + shadedPattern;
    }
}
