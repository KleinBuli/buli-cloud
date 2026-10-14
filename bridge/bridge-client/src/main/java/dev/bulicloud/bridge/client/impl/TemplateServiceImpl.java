package dev.bulicloud.bridge.client.impl;

import dev.bulicloud.bridge.api.service.TemplateService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudTemplate;

import java.util.List;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public class TemplateServiceImpl implements TemplateService {

    private final CloudClient cloudClient;

    public TemplateServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }


    @Override
    public CompletableFuture<Void> create(String groupName, String name) {
        return null;
    }

    @Override
    public CompletableFuture<Void> get(String group, String template) {
        return null;
    }

    @Override
    public CompletableFuture<List<CloudTemplate>> fromGroup(String name) {
        return null;
    }

    @Override
    public CompletableFuture<Void> all() {
        return null;
    }
}
