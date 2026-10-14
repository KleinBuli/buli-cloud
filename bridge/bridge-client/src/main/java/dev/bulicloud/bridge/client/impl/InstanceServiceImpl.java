package dev.bulicloud.bridge.client.impl;

import dev.bulicloud.bridge.api.service.InstanceService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudInstance;

import java.util.List;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public class InstanceServiceImpl implements InstanceService {

    private final CloudClient cloudClient;

    public InstanceServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }


    @Override
    public CompletableFuture<Void> create(String group, String cloudTemplate) {
        return null;
    }

    @Override
    public CompletableFuture<Void> start(String id) {
        return null;
    }

    @Override
    public CompletableFuture<Void> stop(String id) {
        return null;
    }

    @Override
    public CompletableFuture<CloudInstance> get(String id) {
        return null;
    }

    @Override
    public CompletableFuture<List<CloudInstance>> all() {
        return null;
    }
}
