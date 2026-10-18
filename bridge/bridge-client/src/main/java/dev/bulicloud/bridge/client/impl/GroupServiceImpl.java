package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
import dev.bulicloud.bridge.api.service.GroupService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudGroup;
import dev.bulicloud.bridge.model.CloudTemplate;

import java.util.List;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public final class GroupServiceImpl implements GroupService {

    private final CloudClient cloudClient;

    public GroupServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    @Override
    public CompletableFuture<Void> create(String name, dev.bulicloud.bridge.model.enums.GroupRole role) {
        var wireRole = switch (role) {
            case PROXY -> "Proxy";
            case PROXIED_SERVER -> "ProxiedServer";
            case STANDALONE_SERVER -> "StandaloneServer";
        };
        var path = String.format("groups/%s?role=%s", name, wireRole);
        var request  = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> delete(String name) {
        var path = String.format("groups/%s", name);
        var request  = cloudClient.delete(path);

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> setMaintenance(String name, boolean maintenance) {
        var path = String.format("groups/%s/maintenance", name);

        CompletableFuture<Void> request = maintenance
                ? cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {})
                : cloudClient.delete(path);

        return request.whenComplete((result, throwable) -> {
            if (throwable != null) {
                System.err.println(throwable.getMessage());
            }
        });
    }

    @Override
    public CompletableFuture<CloudGroup> get(String name) {
        var path = String.format("groups/%s", name);
        var request  = cloudClient.get(path, new TypeReference<CloudGroup>() {
        });

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<List<String>> fallback() {
        var path = "groups/fallback";
        var request  = cloudClient.get(path, new TypeReference<List<String>>() {
        });

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<List<CloudGroup>> all() {
        var path = "groups";
        var request  = cloudClient.get(path, new TypeReference<List<CloudGroup>>() {
        });

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }
}
