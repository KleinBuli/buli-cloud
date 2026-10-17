package dev.bulicloud.bridge.model;

import dev.bulicloud.bridge.model.enums.ServerSoftware;

@com.fasterxml.jackson.annotation.JsonIgnoreProperties(ignoreUnknown = true)
@com.fasterxml.jackson.databind.annotation.JsonNaming(com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy.class)
public record CloudTemplate(
    ServerSoftware serverSoftware,
    String name,
    @com.fasterxml.jackson.annotation.JsonProperty("custom_server_software_jar_name") String customServerJarName,
    String minecraftVersion,
    int minMemoryMb,
    int maxMemoryMb,
    int minInstances,
    int maxInstances,
    int newInstancesPlayerPercentage,
    boolean staticInstance
) { }
