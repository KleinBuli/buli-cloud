package dev.bulicloud.bridge.model;

public record CloudPlayer(
        String name,
        String uuid,
        String proxy_name,
        String instance_name,
        String ipAddress
) { }
