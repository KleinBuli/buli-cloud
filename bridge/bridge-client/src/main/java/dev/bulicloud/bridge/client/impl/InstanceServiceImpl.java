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

    private static CloudInstance decode(com.fasterxml.jackson.databind.JsonNode info) {
        var instance = info.required("instance");
        return new CloudInstance(instance.required("id").asText(), instance.required("group_name").asText(),
                instance.required("template_name").asText(),
                dev.bulicloud.bridge.model.enums.InstanceMode.valueOf(instance.required("instance_mode").asText().toUpperCase(java.util.Locale.ROOT)));
    }

    @Override
    public CompletableFuture<Void> delete(String id) { return cloudClient.delete("instances/" + id + "/remove"); }

    @Override
    public CompletableFuture<Void> create(String group, String cloudTemplate) {
        var path = String.format("instances/new/%s/%s", group, cloudTemplate);
        var request  = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> start(String id) {
        var path = String.format("instances/%s/start", id);
        var request = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });

    }

    @Override
    public CompletableFuture<Void> stop(String id) {
        var path = String.format("instances/%s/stop", id);
        var request = cloudClient.post(path, java.util.Map.of(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<CloudInstance> get(String id) {
        var path = String.format("instances/%s", id);
        var request = cloudClient.get(path, new TypeReference<com.fasterxml.jackson.databind.JsonNode>() {}).thenApply(InstanceServiceImpl::decode);

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<List<CloudInstance>> all() {
        var path = "instances";
        var request = cloudClient.get(path, new TypeReference<List<com.fasterxml.jackson.databind.JsonNode>>() {})
                .thenApply(nodes -> nodes.stream().map(InstanceServiceImpl::decode).toList());

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }
}
