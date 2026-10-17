package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
import dev.bulicloud.bridge.api.BuliCloudBridge;
import dev.bulicloud.bridge.api.service.CloudPlayerService;
import dev.bulicloud.bridge.api.service.GroupService;
import dev.bulicloud.bridge.api.service.InstanceService;
import dev.bulicloud.bridge.api.service.TemplateService;
import dev.bulicloud.bridge.client.CloudClient;

import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public class BuliCloudBridgeImpl implements BuliCloudBridge {

    private final CloudClient cloudClient;

    public BuliCloudBridgeImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    @Override
    public CompletableFuture<String> health() { return cloudClient.get("health", new TypeReference<String>() {}); }

    @Override
    public CompletableFuture<String> paths() { return cloudClient.get("paths", new TypeReference<String>() {}); }

    private CloudClient getCloudClient() {
        return cloudClient;
    }

    @Override
    public String getVersion() {
        return "0.1.0";
    }

    @Override
    public InstanceService instances() {
        return new InstanceServiceImpl(cloudClient);
    }

    @Override
    public TemplateService templates() {
        return new TemplateServiceImpl(cloudClient);
    }

    @Override
    public GroupService groups() {
        return new GroupServiceImpl(cloudClient);
    }

    @Override
    public CloudPlayerService cloudPlayers() {
        return new CloudPlayerServiceImpl(cloudClient);
    }

    @Override
    public CompletableFuture<Void> reload() {
        var path = "reload";
        var request  = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> shutdown() {
        var path = "shutdown";
        var request  = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }
}
