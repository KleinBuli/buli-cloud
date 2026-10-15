package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
import dev.bulicloud.bridge.api.service.InstanceService;
import dev.bulicloud.bridge.client.CloudClient;
import dev.bulicloud.bridge.model.CloudInstance;

import java.util.List;
import java.util.NoSuchElementException;
import java.util.Objects;
import java.util.concurrent.CompletableFuture;

public class InstanceServiceImpl implements InstanceService {

    private final CloudClient cloudClient;

    public InstanceServiceImpl(CloudClient cloudClient) {
        this.cloudClient = Objects.requireNonNull(cloudClient);
    }

    @Override
    public CompletableFuture<Void> create(String group, String cloudTemplate) {
        var path = String.format("instances/new/%s/%s", group, cloudTemplate);
        var request  = cloudClient.post(path, new Object(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> start(String id) {
        var path = String.format("instances/%s/start", id);
        var request = cloudClient.post(path, new Object(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });

    }

    @Override
    public CompletableFuture<Void> stop(String id) {
        var path = String.format("instances/%s/stop", id);
        var request = cloudClient.post(path, new Object(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<CloudInstance> get(String id) {
        var path = String.format("instances/%s/", id);
        var request = cloudClient.get(path, new TypeReference<CloudInstance>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<List<CloudInstance>> all() {
        var path = "instances";
        var request = cloudClient.post(path, new Object(), new TypeReference<List<CloudInstance>>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }
}
