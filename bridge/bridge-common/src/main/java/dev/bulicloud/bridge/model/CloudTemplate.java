package dev.bulicloud.bridge.model;

import dev.bulicloud.bridge.model.enums.ServerSoftware;

public record CloudTemplate(
    ServerSoftware serverSoftware,
    String name,
    String customServerJarName,
    String minecraftVersion,
    int minMemoryMb,
    int maxMemoryMb,
    int minInstances,
    int maxInstances,
    int newInstancesPlayerPercentage,
    boolean staticInstance
) { }
