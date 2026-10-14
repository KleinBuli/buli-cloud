package dev.bulicloud.bridge.client.impl;

import dev.bulicloud.bridge.api.service.GroupService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudGroup;

import java.util.List;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public final class GroupServiceImpl implements GroupService {

    private final CloudClient cloudClient;

    public GroupServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    @Override
    public CompletableFuture<Void> create(String name) {
        return null;
    }

    @Override
    public CompletableFuture<CloudGroup> get(String name) {
        return null;
    }

    @Override
    public CompletableFuture<List<CloudGroup>> all() {
        return null;
    }
}
