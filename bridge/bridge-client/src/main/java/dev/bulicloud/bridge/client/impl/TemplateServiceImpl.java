package dev.bulicloud.bridge.client.impl;

import com.fasterxml.jackson.core.type.TypeReference;
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
        var path = String.format("templates/new/%s/%s", groupName, name);
        var request  = cloudClient.post(path, new Object(), new TypeReference<Void>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> delete(String groupName, String name) {
        var path = String.format("templates/new/%s/%s", groupName, name);
        var request  = cloudClient.delete(path);

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }


    @Override
    public CompletableFuture<CloudTemplate> get(String group, String template) {
        return fromGroup(group).thenApply(templates ->
                templates.stream()
                        .filter(t -> Objects.equals(t.)))
    }

    @Override
    public CompletableFuture<List<CloudTemplate>> fromGroup(String name) {
        var path = "templates";
        var request  = cloudClient.get(path, new TypeReference<List<CloudTemplate>>() {});

        return request.whenComplete((result, throwable) -> {
            if (throwable != null)
                System.err.println(throwable.getMessage());
        });
    }

    @Override
    public CompletableFuture<Void> all() {
        return null;
    }
}
